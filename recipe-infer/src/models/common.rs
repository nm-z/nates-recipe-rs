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
	gpu_add_into, gpu_bias_add, gpu_broadcast_mul, gpu_concat_into, gpu_copy_into, gpu_gelu_into,
	gpu_layernorm_opt_into, gpu_mul_inplace, gpu_relu_into, gpu_silu_into, gpu_slice_cols,
	gpu_slice_lead_into, gpu_ssm_conv_causal_silu, gpu_ssm_group_rmsnorm, gpu_ssm_scan_mamba1,
	gpu_ssm_scan_mamba2,
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
	pub(super) const fn no_rope(mut self) -> Spec {
		self.rope = false;
		self
	}
}

/// Which selective-SSM mixer a hybrid arch's recurrent layers use.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Recur {
	/// Mamba-1 body (`ssm_x` low-rank split, `ssm_dt` up-projection, optional
	/// dt/B/C RMSNorm), shared with jamba.
	Mamba1,
	/// Mamba-2 grouped-SSD body (fused `in_proj`, conv-carried B/C, per-head
	/// scalar A/D, grouped gated RMSNorm), shared with falcon-h1, granitehybrid,
	/// nemotron_h.
	Mamba2,
	/// plamo2 mixer: mamba-2 head structure (per-head scalar A/D) with mamba-1
	/// projections (`ssm_x` -> B/C/dt, `ssm_dt` up-projection), always-on dt/B/C
	/// RMSNorm, no grouped gated norm, no conv bias, and a per-head-interleaved
	/// z/x split of `in_proj`.
	Plamo2,
}

/// How a hybrid arch lays out its per-layer blocks.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum HyMode {
	/// jamba, granitehybrid: each layer is a mixer sub-block (recurrent OR
	/// attention, chosen per layer) followed by an FFN sub-block, two residuals.
	MixerFfn,
	/// falcon-h1: every layer runs the attention AND mamba-2 branches on the same
	/// normed input, sums them, adds the residual, then an FFN sub-block.
	Parallel,
	/// nemotron_h: each layer is EXACTLY ONE body (mamba-2, attention, or FFN),
	/// one residual; the triage is by tensor presence.
	Triage,
	/// plamo2: sandwich norms wrap both halves. Pre-norm, mixer, post-norm, then a
	/// residual; then pre-ff-norm, SwiGLU FFN, post-ff-norm, then a residual.
	Sandwich,
}

/// A per-layer attention/recurrent-interleaving composition. `sp` drives both the
/// attention branch (rope/bias/qk-norm/o-bias) and the FFN branch (activation +
/// bias); `recur` picks the SSM mixer for recurrent layers; `mode` the block
/// layout. Per-layer routing is by tensor presence (`ssm_in` vs `q_proj`), the
/// honest signal for which mixer a block carries.
#[derive(Clone, Copy)]
pub(super) struct Hy {
	pub recur: Recur,
	pub sp: Spec,
	pub mode: HyMode,
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
	// the F32 reference applies o_proj/ffn biases but not q/k/v (qwen2/phi2 regress with them), so attn_bias stays a no-op
	let _ = sp.attn_bias;
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
			if sp.ffn_bias
				&& let Some(b) = &m.ffn_gate_bias[l]
			{
				gpu_bias_add(&ar.g, b, t, nff, &ar.g)?;
			}
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
			if sp.ffn_bias
				&& let Some(b) = &m.ffn_gate_bias[l]
			{
				gpu_bias_add(&ar.g, b, t, nff, &ar.g)?;
			}
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

/// The named per-layer SSM parameter buffer for layer `l`, or an error naming
/// the missing tensor (a load gap fails that model's row, never panics a sweep).
fn ssm_of<'m>(
	buf: &'m [Option<GpuBuffer>],
	l: usize,
	arch: &str,
	name: &str,
) -> Result<&'m GpuBuffer> {
	buf.get(l)
		.and_then(|o| o.as_ref())
		.ok_or_else(|| anyhow!("{arch}: layer {l} has no {name} SSM tensor"))
}

/// Mamba-1 selective-SSM decoder block (llama.cpp mamba-base.cpp:7-147): block
/// pre-norm, `in_proj` split into `x`/`z`, causal depthwise conv + SiLU on `x`,
/// the `ssm_x` low-rank projection split into `dt`/`B`/`C`, the `ssm_dt`
/// up-projection with bias, the fused selective scan (softplus/exp/B/C/D folded
/// in [`gpu_ssm_scan_mamba1`]), the `SiLU(z)` gate, `out_proj`, and the residual.
/// No FFN and no attention: the block IS the mixer. All scratch is arena-resident,
/// so the decode loop allocates nothing.
pub(super) fn layer_mamba(
	m: &Model,
	l: usize,
	h_in: &GpuBuffer,
	h_out: &GpuBuffer,
	t: usize,
	ar: &Arena,
) -> Result<()> {
	mamba1_mix(m, l, h_in, &ar.o, t, ar)?;
	gpu_add_into(&ar.o, h_in, t * m.hp.ne, h_out)?;
	return Ok(());
}

/// The mamba-1 mixer body: block pre-norm through `out_proj`, writing the
/// projected output to `out` WITHOUT the block residual (the caller adds it).
/// The optional dt/B/C RMSNorm (jamba, plamo2) applies to the low-rank
/// `dt`/`B`/`C` by presence, before the `ssm_dt` up-projection, matching
/// `llama.cpp` mamba-base.cpp:97-101; a plain mamba layer ships none and runs
/// the identical numeric path it did before.
fn mamba1_mix(
	m: &Model,
	l: usize,
	h_in: &GpuBuffer,
	out: &GpuBuffer,
	t: usize,
	ar: &Arena,
) -> Result<()> {
	let hp = &m.hp;
	let arch = hp.arch.as_str();
	let ne = hp.ne;
	let di = hp.ssm_d_inner;
	let ds = hp.ssm_d_state;
	let dr = hp.ssm_dt_rank;
	let dc = hp.ssm_d_conv;
	let dbw = dr + 2 * ds;
	if hp.ssm_n_group != 1 {
		return Err(anyhow!(
			"{arch}: mamba-1 block expects a single selective group, got n_group={}",
			hp.ssm_n_group
		));
	}

	gpu_rmsnorm_f64(h_in, norm_of(m, l, "input")?, &m.eps, t, ne, &ar.x)?;

	let w_in = m.stream(&layer_name(l, "self_attn.ssm_in.weight"))?;
	gpu_gemm_bt_f64(&ar.x, &w_in.view(0, di * ne), t, di, ne, &ar.ss_x)?;
	gpu_gemm_bt_f64(&ar.x, &w_in.view(di * ne, di * ne), t, di, ne, &ar.ss_z)?;

	let conv_w = ssm_of(&m.ssm_conv_w, l, arch, "ssm_conv1d.weight")?;
	let conv_b = ssm_of(&m.ssm_conv_b, l, arch, "ssm_conv1d.bias")?;
	gpu_ssm_conv_causal_silu(&ar.ss_x, conv_w, conv_b, t, di, dc, &ar.ss_xc)?;

	let w_x = m.stream(&layer_name(l, "self_attn.ssm_x.weight"))?;
	gpu_gemm_bt_f64(&ar.ss_xc, &w_x, t, dbw, di, &ar.ss_db)?;
	gpu_slice_cols(&ar.ss_db, t, dbw, 0, dr, &ar.ss_dtlr)?;
	gpu_slice_cols(&ar.ss_db, t, dbw, dr, ds, &ar.ss_bb)?;
	gpu_slice_cols(&ar.ss_db, t, dbw, dr + ds, ds, &ar.ss_cc)?;

	if let Some(g) = &m.ssm_dt_norm[l] {
		gpu_rmsnorm_f64(&ar.ss_dtlr, g, &m.eps, t, dr, &ar.ss_dtlr)?;
	}
	if let Some(g) = &m.ssm_b_norm[l] {
		gpu_rmsnorm_f64(&ar.ss_bb, g, &m.eps, t, ds, &ar.ss_bb)?;
	}
	if let Some(g) = &m.ssm_c_norm[l] {
		gpu_rmsnorm_f64(&ar.ss_cc, g, &m.eps, t, ds, &ar.ss_cc)?;
	}

	let w_dt = m.stream(&layer_name(l, "self_attn.ssm_dt.weight"))?;
	gpu_gemm_bt_f64(&ar.ss_dtlr, &w_dt, t, di, dr, &ar.ss_dt)?;
	let dt_b = ssm_of(&m.ssm_dt_b, l, arch, "ssm_dt.bias")?;
	gpu_bias_add(&ar.ss_dt, dt_b, t, di, &ar.ss_dt)?;

	let a = ssm_of(&m.ssm_a, l, arch, "ssm_a")?;
	let d = ssm_of(&m.ssm_d, l, arch, "ssm_d")?;
	gpu_ssm_scan_mamba1(&ar.ss_xc, &ar.ss_dt, a, &ar.ss_bb, &ar.ss_cc, d, t, di, ds, &ar.ss_y)?;

	gpu_silu_into(&ar.ss_z, t * di, &ar.ss_z)?;
	gpu_mul_inplace(&ar.ss_z, t * di, &ar.ss_y)?;

	let w_out = m.stream(&layer_name(l, "self_attn.ssm_out.weight"))?;
	gpu_gemm_bt_f64(&ar.ss_y, &w_out, t, ne, di, out)?;
	return Ok(());
}

/// Mamba-2 grouped-SSM (SSD) decoder block (llama.cpp mamba-base.cpp:149-288):
/// block pre-norm, one `in_proj` split into `z`/`xBC`/`dt`, a causal depthwise
/// conv + SiLU over the WHOLE `xBC` (conv_dim = d_inner + 2*n_group*d_state), the
/// per-head dt bias, the fused grouped selective scan (per-head scalar `A`/`D`,
/// per-group `B`/`C` read out of the conv output by offset, D skip folded in),
/// the `SiLU(z)` gate, the grouped gated RMSNorm (gate THEN norm), `out_proj`,
/// and the residual. No dt/x projections (dt rides `in_proj`, B/C ride the conv);
/// no FFN and no attention. All scratch is arena-resident.
pub(super) fn layer_mamba2(
	m: &Model,
	l: usize,
	h_in: &GpuBuffer,
	h_out: &GpuBuffer,
	t: usize,
	ar: &Arena,
) -> Result<()> {
	mamba2_mix(m, l, h_in, &ar.o, t, ar)?;
	gpu_add_into(&ar.o, h_in, t * m.hp.ne, h_out)?;
	return Ok(());
}

/// The mamba-2 mixer body: block pre-norm through `out_proj`, writing the
/// projected output to `out` WITHOUT the block residual (the caller adds it).
/// Shared verbatim by the mamba-2 hybrids (falcon-h1, granitehybrid,
/// nemotron_h) and the pure `mamba2` arch.
fn mamba2_mix(
	m: &Model,
	l: usize,
	h_in: &GpuBuffer,
	out: &GpuBuffer,
	t: usize,
	ar: &Arena,
) -> Result<()> {
	let hp = &m.hp;
	let arch = hp.arch.as_str();
	let ne = hp.ne;
	let di = hp.ssm_d_inner;
	let ds = hp.ssm_d_state;
	let ng = hp.ssm_n_group;
	let dc = hp.ssm_d_conv;
	let nh = hp.ssm_dt_rank;
	let conv_dim = di + 2 * ng * ds;
	if nh == 0 || di % nh != 0 {
		return Err(anyhow!("{arch}: mamba-2 d_inner={di} not divisible by n_head={nh}"));
	}
	if ng == 0 || di % ng != 0 {
		return Err(anyhow!("{arch}: mamba-2 d_inner={di} not divisible by n_group={ng}"));
	}

	gpu_rmsnorm_f64(h_in, norm_of(m, l, "input")?, &m.eps, t, ne, &ar.x)?;

	let w_in = m.stream(&layer_name(l, "self_attn.ssm_in.weight"))?;
	gpu_gemm_bt_f64(&ar.x, &w_in.view(0, di * ne), t, di, ne, &ar.ss_z)?;
	gpu_gemm_bt_f64(&ar.x, &w_in.view(di * ne, conv_dim * ne), t, conv_dim, ne, &ar.ss_xbc)?;
	gpu_gemm_bt_f64(&ar.x, &w_in.view((di + conv_dim) * ne, nh * ne), t, nh, ne, &ar.ss_dtlr)?;

	let conv_w = ssm_of(&m.ssm_conv_w, l, arch, "ssm_conv1d.weight")?;
	let conv_b = ssm_of(&m.ssm_conv_b, l, arch, "ssm_conv1d.bias")?;
	gpu_ssm_conv_causal_silu(&ar.ss_xbc, conv_w, conv_b, t, conv_dim, dc, &ar.ss_xbcc)?;

	let dt_b = ssm_of(&m.ssm_dt_b, l, arch, "ssm_dt.bias")?;
	gpu_bias_add(&ar.ss_dtlr, dt_b, t, nh, &ar.ss_dtlr)?;

	let a = ssm_of(&m.ssm_a, l, arch, "ssm_a")?;
	let d = ssm_of(&m.ssm_d, l, arch, "ssm_d")?;
	gpu_ssm_scan_mamba2(&ar.ss_xbcc, &ar.ss_dtlr, a, d, t, di, ds, nh, ng, conv_dim, &ar.ss_y)?;

	gpu_silu_into(&ar.ss_z, t * di, &ar.ss_z)?;
	gpu_mul_inplace(&ar.ss_z, t * di, &ar.ss_y)?;
	let ssm_norm = ssm_of(&m.ssm_norm, l, arch, "ssm_norm.weight")?;
	gpu_ssm_group_rmsnorm(&ar.ss_y, ssm_norm, &m.eps, t * ng, di / ng, ng, &ar.ss_y)?;

	let w_out = m.stream(&layer_name(l, "self_attn.ssm_out.weight"))?;
	gpu_gemm_bt_f64(&ar.ss_y, &w_out, t, ne, di, out)?;
	return Ok(());
}

/// De-interleaves one of the two per-head sub-vectors from a `[t, 2*head_dim*
/// n_head]` tensor whose per-head layout is `[first_half | second_half]`, into a
/// contiguous `[t, head_dim*n_head]` `out`. `within_off` selects the half (`0`
/// for the first, `head_dim` for the second). `sa`/`sb` are `[t, d_inner]`
/// scratch. plamo2's `in_proj` output splits `z`/`x` this way (plamo2.cpp:293-310).
fn deinterleave_heads(
	zx: &GpuBuffer,
	t: usize,
	head_dim: usize,
	n_head: usize,
	within_off: usize,
	out: &GpuBuffer,
	sa: &GpuBuffer,
	sb: &GpuBuffer,
) -> Result<()> {
	let total = 2 * head_dim * n_head;
	gpu_slice_cols(zx, t, total, within_off, head_dim, out)?;
	for h in 1..n_head {
		gpu_slice_cols(zx, t, total, h * 2 * head_dim + within_off, head_dim, sa)?;
		gpu_concat_into(out, sa, t, h * head_dim, head_dim, sb)?;
		gpu_copy_into(sb, t * (h + 1) * head_dim, out)?;
	}
	return Ok(());
}

/// The plamo2 mixer body (plamo2.cpp:258-426), writing `out_proj` to `out`
/// without the block residual. mamba-2 head scan (per-head scalar A/D, single
/// shared B/C) fed by mamba-1 front-end projections: per-head z/x de-interleave
/// of `in_proj`, causal conv + SiLU (no bias), `ssm_x` -> `[B|C|dt]`, always-on
/// dt/B/C RMSNorm, `ssm_dt` up-projection, then the packed grouped scan reused
/// from mamba-2 (`n_group == 1`). No grouped gated RMSNorm.
fn plamo2_mix(
	m: &Model,
	l: usize,
	h_in: &GpuBuffer,
	out: &GpuBuffer,
	t: usize,
	ar: &Arena,
) -> Result<()> {
	let hp = &m.hp;
	let arch = hp.arch.as_str();
	let ne = hp.ne;
	let di = hp.ssm_d_inner;
	let ds = hp.ssm_d_state;
	let nh = hp.ssm_dt_rank;
	let dc = hp.ssm_d_conv;
	let dtd = hp.ssm_dt_dim;
	if nh == 0 || di % nh != 0 {
		return Err(anyhow!("{arch}: plamo2 d_inner={di} not divisible by n_head={nh}"));
	}
	let head_dim = di / nh;
	let bcd = dtd + 2 * ds;
	let conv_dim = di + 2 * ds;

	gpu_rmsnorm_f64(h_in, norm_of(m, l, "input")?, &m.eps, t, ne, &ar.x)?;

	let w_in = m.stream(&layer_name(l, "self_attn.ssm_in.weight"))?;
	gpu_gemm_bt_f64(&ar.x, &w_in, t, 2 * di, ne, &ar.ss_zx)?;
	deinterleave_heads(&ar.ss_zx, t, head_dim, nh, head_dim, &ar.ss_x, &ar.ss_dt, &ar.ss_y)?;
	deinterleave_heads(&ar.ss_zx, t, head_dim, nh, 0, &ar.ss_z, &ar.ss_dt, &ar.ss_y)?;

	let conv_w = ssm_of(&m.ssm_conv_w, l, arch, "ssm_conv1d.weight")?;
	let conv_b = ssm_of(&m.ssm_conv_b, l, arch, "ssm_conv1d.bias")?;
	gpu_ssm_conv_causal_silu(&ar.ss_x, conv_w, conv_b, t, di, dc, &ar.ss_xc)?;

	let w_x = m.stream(&layer_name(l, "self_attn.ssm_x.weight"))?;
	gpu_gemm_bt_f64(&ar.ss_xc, &w_x, t, bcd, di, &ar.ss_db)?;
	gpu_slice_cols(&ar.ss_db, t, bcd, 0, ds, &ar.ss_bb)?;
	gpu_slice_cols(&ar.ss_db, t, bcd, ds, ds, &ar.ss_cc)?;
	gpu_slice_cols(&ar.ss_db, t, bcd, 2 * ds, dtd, &ar.ss_dtlr)?;

	gpu_rmsnorm_f64(&ar.ss_bb, ssm_of(&m.ssm_b_norm, l, arch, "ssm_b_norm")?, &m.eps, t, ds, &ar.ss_bb)?;
	gpu_rmsnorm_f64(&ar.ss_cc, ssm_of(&m.ssm_c_norm, l, arch, "ssm_c_norm")?, &m.eps, t, ds, &ar.ss_cc)?;
	gpu_rmsnorm_f64(&ar.ss_dtlr, ssm_of(&m.ssm_dt_norm, l, arch, "ssm_dt_norm")?, &m.eps, t, dtd, &ar.ss_dtlr)?;

	let dt = ar.ss_dt.view(0, t * nh);
	let w_dt = m.stream(&layer_name(l, "self_attn.ssm_dt.weight"))?;
	gpu_gemm_bt_f64(&ar.ss_dtlr, &w_dt, t, nh, dtd, &dt)?;
	gpu_bias_add(&dt, ssm_of(&m.ssm_dt_b, l, arch, "ssm_dt.bias")?, t, nh, &dt)?;

	gpu_concat_into(&ar.ss_bb, &ar.ss_cc, t, ds, ds, &ar.ss_db)?;
	gpu_concat_into(&ar.ss_xc, &ar.ss_db.view(0, t * 2 * ds), t, di, 2 * ds, &ar.ss_xbc)?;

	let a = ssm_of(&m.ssm_a, l, arch, "ssm_a")?;
	let d = ssm_of(&m.ssm_d, l, arch, "ssm_d")?;
	gpu_ssm_scan_mamba2(&ar.ss_xbc, &dt, a, d, t, di, ds, nh, 1, conv_dim, &ar.ss_y)?;

	gpu_silu_into(&ar.ss_z, t * di, &ar.ss_z)?;
	gpu_mul_inplace(&ar.ss_z, t * di, &ar.ss_y)?;

	let w_out = m.stream(&layer_name(l, "self_attn.ssm_out.weight"))?;
	gpu_gemm_bt_f64(&ar.ss_y, &w_out, t, ne, di, out)?;
	return Ok(());
}

/// The plamo2 attention body (plamo2.cpp:203-256): pre-norm, fused-QKV split
/// (via [`synth_qkv_slices`]), per-head Q/K RMSNorm (per-`(head,dim)` gamma),
/// RoPE, GQA, `o_proj`. Writes the projection to `out` without the block
/// residual (the sandwich wrapper adds it). No output bias.
fn plamo2_attn(
	m: &Model,
	l: usize,
	h_in: &GpuBuffer,
	out: &GpuBuffer,
	t: usize,
	ar: &Arena,
	attn_scale: &GpuBuffer,
) -> Result<()> {
	let hp = &m.hp;
	let ne = hp.ne;
	let d = &hp.dims[l];
	let nqh = d.nqh;
	let (hd, nkv, qd, kd) = (d.hd, d.nkv, nqh * d.hd, d.nkv * d.hd);
	gpu_rmsnorm_f64(h_in, norm_of(m, l, "input")?, &m.eps, t, ne, &ar.x)?;
	gpu_gemm_bt_f64(&ar.x, &m.stream(&layer_name(l, "self_attn.q_proj.weight"))?, t, qd, ne, &ar.q)?;
	gpu_gemm_bt_f64(&ar.x, &m.stream(&layer_name(l, "self_attn.k_proj.weight"))?, t, kd, ne, &ar.k)?;
	gpu_gemm_bt_f64(&ar.x, &m.stream(&layer_name(l, "self_attn.v_proj.weight"))?, t, kd, ne, &ar.v)?;
	gpu_rmsnorm_f64_nogamma(&ar.q, &m.eps, t * nqh, hd, &ar.q)?;
	gpu_broadcast_mul(&ar.q, norm_of(m, l, "q_norm")?, t * qd, qd, &ar.q)?;
	gpu_rmsnorm_f64_nogamma(&ar.k, &m.eps, t * nkv, hd, &ar.k)?;
	gpu_broadcast_mul(&ar.k, norm_of(m, l, "k_norm")?, t * kd, kd, &ar.k)?;
	gpu_rope_partial(&m.theta_full, t * nqh, hd, hd, nqh, &ar.q)?;
	gpu_rope_partial(&m.theta_full, t * nkv, hd, hd, nkv, &ar.k)?;
	gpu_scale_f64_inplace(attn_scale, t * qd, &ar.q)?;
	gpu_gqa_attn(&ar.q, &ar.k, &ar.v, t, nqh, nkv, hd, t, 0.0, &ar.attn)?;
	gpu_gemm_bt_f64(&ar.attn, &m.stream(&layer_name(l, "self_attn.o_proj.weight"))?, t, ne, qd, out)?;
	return Ok(());
}

/// True if layer `l` carries a recurrent SSM mixer (its `ssm_in` projection is
/// present) rather than attention: the honest per-layer interleave signal, the
/// same discriminator `llama.cpp` derives from `n_head_kv(l)==0`.
fn layer_is_recur(m: &Model, l: usize) -> bool {
	return m.big.contains_key(&layer_name(l, "self_attn.ssm_in.weight"));
}

/// True if layer `l` carries an attention mixer (its `q_proj` is present, fused
/// or separate). A nemotron FFN-only layer has neither this nor `ssm_in`.
fn layer_is_attn(m: &Model, l: usize) -> bool {
	return m.big.contains_key(&layer_name(l, "self_attn.q_proj.weight"));
}

/// The FFN half of a hybrid block: pre-`norm_key` norm of `src` into `ar.cms`,
/// the dense FFN into `ar.mlp0`, residual `resid` into `h_out`. Per-layer MoE
/// FFN (real jamba/granite/nemotron-MoE checkpoints) is not wired; the parity
/// fixtures are all dense (`expert_count == 0`), so a MoE layer fails clean here.
fn hybrid_ffn(
	m: &Model,
	l: usize,
	sp: &Spec,
	norm_key: &str,
	src: &GpuBuffer,
	resid: &GpuBuffer,
	h_out: &GpuBuffer,
	t: usize,
	ar: &Arena,
) -> Result<()> {
	let ne = m.hp.ne;
	if m.layer_is_moe(l) {
		return Err(anyhow!(
			"{}: hybrid per-layer MoE FFN not wired (fixtures are dense)",
			m.hp.arch
		));
	}
	blk_norm(m, sp, l, norm_key, t, ne, src, &ar.cms)?;
	ffn(m, l, sp, t, ar, &ar.cms)?;
	gpu_add_into(&ar.mlp0, resid, t * ne, h_out)?;
	return Ok(());
}

/// Per-layer attention/recurrent-interleaving decoder block for the mamba
/// hybrids. Routes each layer to its mixer by tensor presence and composes the
/// block per `hy.mode`, reusing the verified [`attn_block`], [`mamba1_mix`],
/// [`mamba2_mix`], and [`ffn`] drivers 1:1 with the arch's `llama.cpp` graph.
pub(super) fn layer_hybrid(
	m: &Model,
	l: usize,
	hy: &Hy,
	h_in: &GpuBuffer,
	h_out: &GpuBuffer,
	t: usize,
	ar: &Arena,
	attn_scale: &GpuBuffer,
) -> Result<()> {
	let ne = m.hp.ne;
	let recur_mix = |out: &GpuBuffer| -> Result<()> {
		return match hy.recur {
			Recur::Mamba1 => mamba1_mix(m, l, h_in, out, t, ar),
			Recur::Mamba2 => mamba2_mix(m, l, h_in, out, t, ar),
			Recur::Plamo2 => plamo2_mix(m, l, h_in, out, t, ar),
		};
	};
	match hy.mode {
		HyMode::Parallel => {
			attn_block(m, l, &hy.sp, h_in, t, ar, attn_scale)?;
			mamba2_mix(m, l, h_in, &ar.o, t, ar)?;
			gpu_add_into(&ar.attn_out, &ar.o, t * ne, &ar.mlp)?;
			hybrid_ffn(m, l, &hy.sp, "pre_ff", &ar.mlp, &ar.mlp, h_out, t, ar)?;
		}
		HyMode::MixerFfn => {
			if layer_is_recur(m, l) {
				recur_mix(&ar.o)?;
				gpu_add_into(&ar.o, h_in, t * ne, &ar.attn_out)?;
			} else {
				attn_block(m, l, &hy.sp, h_in, t, ar, attn_scale)?;
			}
			hybrid_ffn(m, l, &hy.sp, "pre_ff", &ar.attn_out, &ar.attn_out, h_out, t, ar)?;
		}
		HyMode::Triage => {
			if layer_is_recur(m, l) {
				recur_mix(&ar.o)?;
				gpu_add_into(&ar.o, h_in, t * ne, h_out)?;
			} else if layer_is_attn(m, l) {
				attn_block(m, l, &hy.sp, h_in, t, ar, attn_scale)?;
				gpu_copy_into(&ar.attn_out, t * ne, h_out)?;
			} else {
				hybrid_ffn(m, l, &hy.sp, "input", h_in, h_in, h_out, t, ar)?;
			}
		}
		HyMode::Sandwich => {
			if layer_is_recur(m, l) {
				recur_mix(&ar.o)?;
			} else {
				plamo2_attn(m, l, h_in, &ar.o, t, ar, attn_scale)?;
			}
			gpu_rmsnorm_f64(&ar.o, norm_of(m, l, "post_attn")?, &m.eps, t, ne, &ar.o)?;
			gpu_add_into(&ar.o, h_in, t * ne, &ar.attn_out)?;
			blk_norm(m, &hy.sp, l, "pre_ff", t, ne, &ar.attn_out, &ar.cms)?;
			ffn(m, l, &hy.sp, t, ar, &ar.cms)?;
			gpu_rmsnorm_f64(&ar.mlp0, norm_of(m, l, "pfw")?, &m.eps, t, ne, &ar.mlp0)?;
			gpu_add_into(&ar.mlp0, &ar.attn_out, t * ne, h_out)?;
		}
	}
	return Ok(());
}
