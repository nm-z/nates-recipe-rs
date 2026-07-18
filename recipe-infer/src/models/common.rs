//! Shared composition vocabulary for the per-architecture decode ports. Every
//! `models/<arch>.rs` is one architecture: it declares that arch's [`Spec`] and
//! delegates to [`layer_spec`] (dense) or [`layer_moe`] (mixture-of-experts),
//! which compose the shared `gpu_*` kernels 1:1 with the arch's `llama.cpp`
//! `build_arch_graph`. Recurrent families spell out their own composition.

use super::super::{Arena, Model, Nk, layer_name, softmax};
use super::DecCtx;
use anyhow::{Result, anyhow};
use gpu_core::infer_ops::{
	gpu_flash_gqa, gpu_flash_mla, gpu_gemm_bt_f64, gpu_glu_silu, gpu_rmsnorm_f64,
	gpu_rmsnorm_f64_nogamma, gpu_rope_partial, gpu_rope_partial_factors,
	gpu_rope_partial_factors_pos, gpu_rope_partial_pos, gpu_scale_f64_inplace,
};
use gpu_core::k_gapact::gpu_softplus;
use gpu_core::kernels::{
	gpu_add_into, gpu_bias_add, gpu_broadcast_mul, gpu_concat_into, gpu_copy_into,
	gpu_gated_delta_scan, gpu_gelu_into, gpu_l2norm_rows, gpu_layernorm_opt_into, gpu_mul_inplace,
	gpu_relu_into, gpu_row_scale, gpu_sigmoid_into, gpu_silu_into, gpu_slice_cols,
	gpu_slice_lead_into, gpu_ssm_conv_causal, gpu_ssm_conv_causal_silu, gpu_ssm_group_rmsnorm,
	gpu_ssm_scan_mamba1, gpu_ssm_scan_mamba2,
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
	/// A hardcoded input-embedding scale the arch's graph applies as a literal
	/// constant (minicpm3 `scale_embd=12`); `1.0` means no constant scale.
	pub emb_scale_const: f64,
	/// talkie: a non-parametric RMS of the initial embedding is retained and added
	/// (scaled by each layer's `layer_output_scale`) to every layer's output. Drives
	/// the decode-loop embed prenorm + stash and the [`layer_talkie`] skip residual.
	pub embd_skip: bool,
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
			emb_scale_const: 1.0,
			embd_skip: false,
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
	pub(super) const fn emb_scale_const(mut self, v: f64) -> Spec {
		self.emb_scale_const = v;
		self
	}
	pub(super) const fn embd_skip(mut self) -> Spec {
		self.embd_skip = true;
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
	/// Gated Delta-Net (GDA) linear-attention mixer (qwen3.5/next/moe): q/k/v/z
	/// projections, causal short conv + SiLU, per-head L2-normed q/k, a per-head
	/// scalar log-decay, the delta-rule scan, and a SiLU(z)-gated RMSNorm output.
	GatedDelta,
	/// Kimi Delta-Attention (KDA) mixer (kimi-linear): separate q/k/v projections
	/// each with its own causal conv, a per-CHANNEL log-decay LoRA, the delta-rule
	/// scan, and a sigmoid(`g_b(g_a(x))`)-gated RMSNorm output.
	Kda,
	/// lfm2 gated short-convolution mixer: operator-norm, `in_proj` split into
	/// `B`/`C`/`x`, the `B*x` pre-conv gate, a causal depthwise conv, the `C*conv`
	/// post-conv gate, and `out_proj`.
	ShortConv,
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
	/// qwen3.5/next/moe, kimi-linear: each layer runs one mixer (delta recurrent
	/// OR full attention, chosen by conv-tensor presence) writing its projected
	/// output, adds the block residual, then a `pre_ff`-normed FFN sub-block (dense
	/// SwiGLU or MoE + shared expert) with its own residual.
	DeltaNet,
	/// lfm2/lfm2moe: each layer runs one mixer (short-conv recurrent OR SWA
	/// attention, chosen by shortconv-tensor presence) on the operator-normed input,
	/// adds the block residual, then an `ffn_norm`-normed FFN sub-block (dense SwiGLU
	/// or sigmoid-gated MoE) with its own residual.
	ShortConv,
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
fn norm_of<'m>(m: &'m Model, l: usize, key: Nk) -> Result<&'m GpuBuffer> {
	m.norms[l][key as usize]
		.as_ref()
		.ok_or_else(|| anyhow!("{}: layer {l} has no {:?} norm weight", m.hp.arch, key.name()))
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
	key: Nk,
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
	let beta = m.norms_b[l][key as usize].as_ref();
	return apply_norm(sp.norm, gamma, beta, &m.eps, rows, cols, x, out);
}

/// This layer's recurrent state buffer — variant-checked by [`crate::llm::LayerCache`],
/// so a scan mixer on a non-scan layer is a loud structural error, never a recompute.
fn rec_of<'a>(dec: &DecCtx<'a>) -> Result<&'a GpuBuffer> {
	return dec.state.rec();
}

/// The `(read, write)` conv-window buffers for conv `i` of this layer's mixer.
fn conv_io<'a>(dec: &DecCtx<'a>, i: usize) -> Result<(&'a GpuBuffer, &'a GpuBuffer)> {
	return dec.state.conv_io(i);
}

/// NeoX RoPE with optional LongRoPE factors AND a position base, so a cached decode
/// rotates the NEW rows at their true absolute positions (`pos_base = cached`).
fn rope_maybe_factors_pos(
	theta: &GpuBuffer,
	factors: Option<&GpuBuffer>,
	rows: usize,
	head_dim: usize,
	rot: usize,
	heads: usize,
	pos_base: usize,
	buf: &GpuBuffer,
) -> Result<()> {
	match factors {
		Some(f) => gpu_rope_partial_factors_pos(theta, rows, head_dim, rot, heads, f, pos_base, buf)?,
		None => gpu_rope_partial_pos(theta, rows, head_dim, rot, heads, pos_base, buf)?,
	}
	return Ok(());
}

/// The ONE attention walk every GQA site resolves through: append the new K/V
/// into the resident window, then walk the whole causal range in ASCENDING
/// absolute-key order — host-tier segments `[0, win_base)` staged back in
/// first, then the resident window — carrying the online-softmax `(m, l, acc)`
/// across launches and normalizing only on the last. A fully-resident cache is
/// the `win_base == 0` case: the host loop never runs and the resident launch
/// covers everything — the SAME code path, not a branch. By the flash carry
/// contract the walk is BIT-IDENTICAL to one launch over the full contiguous
/// cache. `kd`/`vd` are the cache's per-row K/V widths (equal everywhere
/// except minicpm3's naive MLA).
fn cached_gqa(
	d: &DecCtx,
	ar: &Arena,
	q: &GpuBuffer,
	new_k: &GpuBuffer,
	new_v: &GpuBuffer,
	t: usize,
	nqh: usize,
	nkv: usize,
	hd: usize,
	kd: usize,
	vd: usize,
	max_bias: f64,
	bidir: bool,
	out: &GpuBuffer,
) -> Result<()> {
	let s = d.state.kv()?;
	let es = super::super::FWD_DT.elem_size();
	let total = d.cached + t;
	gpu_copy_into(new_k, t * kd, &s.k.view((d.cached - d.win_base) * kd, t * kd))?;
	gpu_copy_into(new_v, t * vd, &s.v.view((d.cached - d.win_base) * vd, t * vd))?;
	let causal_below = if bidir { 0 } else { total };
	let stage_k = [&d.stage.sk[0], &d.stage.sk[1]];
	let stage_v = [&d.stage.sv[0], &d.stage.sv[1]];
	let mut scratch = Vec::new();
	let mut seg = 0usize;
	let mut buf = 0usize;
	while seg < d.win_base {
		let n = d.win.min(d.win_base - seg);
		s.hk.stage_into(seg * kd * es, n * kd * es, stage_k[buf], &mut scratch)?;
		s.hv.stage_into(seg * vd * es, n * vd * es, stage_v[buf], &mut scratch)?;
		gpu_flash_gqa(
			q, stage_k[buf], stage_v[buf], t, n, nqh, nkv, hd, max_bias, d.cached, causal_below, out,
			&ar.cm, &ar.cl, &ar.cacc, seg, false,
		)?;
		seg += n;
		buf ^= 1;
	}
	let res_n = total - d.win_base;
	gpu_flash_gqa(
		q, &s.k, &s.v, t, res_n, nqh, nkv, hd, max_bias, d.cached, causal_below, out,
		&ar.cm, &ar.cl, &ar.cacc, d.win_base, true,
	)?;
	return Ok(());
}

/// [`cached_gqa`]'s MLA twin (asymmetric widths: `kw` latent+pe keys, `kvlr`
/// latent values; always causal), shared by the absorbed deepseek2 and kimi blocks.
fn cached_mla(
	d: &DecCtx,
	ar: &Arena,
	q: &GpuBuffer,
	new_k: &GpuBuffer,
	new_v: &GpuBuffer,
	t: usize,
	nqh: usize,
	kw: usize,
	kvlr: usize,
	out: &GpuBuffer,
) -> Result<()> {
	let s = d.state.kv()?;
	let es = super::super::FWD_DT.elem_size();
	let total = d.cached + t;
	gpu_copy_into(new_k, t * kw, &s.k.view((d.cached - d.win_base) * kw, t * kw))?;
	gpu_copy_into(new_v, t * kvlr, &s.v.view((d.cached - d.win_base) * kvlr, t * kvlr))?;
	let stage_k = [&d.stage.sk[0], &d.stage.sk[1]];
	let stage_v = [&d.stage.sv[0], &d.stage.sv[1]];
	let mut scratch = Vec::new();
	let mut seg = 0usize;
	let mut buf = 0usize;
	while seg < d.win_base {
		let n = d.win.min(d.win_base - seg);
		s.hk.stage_into(seg * kw * es, n * kw * es, stage_k[buf], &mut scratch)?;
		s.hv.stage_into(seg * kvlr * es, n * kvlr * es, stage_v[buf], &mut scratch)?;
		gpu_flash_mla(
			q, stage_k[buf], stage_v[buf], t, n, nqh, 1, kw, kvlr, d.cached, total, out,
			&ar.cm, &ar.cl, &ar.cacc, seg, false,
		)?;
		seg += n;
		buf ^= 1;
	}
	let res_n = total - d.win_base;
	gpu_flash_mla(
		q, &s.k, &s.v, t, res_n, nqh, 1, kw, kvlr, d.cached, total, out,
		&ar.cm, &ar.cl, &ar.cacc, d.win_base, true,
	)?;
	return Ok(());
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
	dec: &DecCtx,
) -> Result<()> {
	if sp.mla {
		return mla_attn_block(m, l, sp, h_in, t, ar, dec);
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
		blk_norm(m, sp, l, Nk::Input, t, ne, h_in, &ar.x)?;
		if sp.second_attn_norm && m.norms[l][Nk::Attn2 as usize].is_some() {
			blk_norm(m, sp, l, Nk::Attn2, t, ne, h_in, &ar.cms)?;
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
	// the F32 reference applies o_proj/ffn biases but not q/k/v (qwen2/phi2/lfm2/talkie regress with them, finding 20), so attn_bias stays a no-op
	let _ = sp.attn_bias;
	if sp.qk_norm {
		gpu_rmsnorm_f64(&ar.q, norm_of(m, l, Nk::QNorm)?, &m.eps, t * nqh, hd, &ar.q)?;
		gpu_rmsnorm_f64(&ar.k, norm_of(m, l, Nk::KNorm)?, &m.eps, t * nkv, hd, &ar.k)?;
	}
	let pos_base = dec.cached;
	if sp.rope {
		gpu_rope_partial_pos(theta, t * nqh, hd, hd, nqh, pos_base, &ar.q)?;
		gpu_rope_partial_pos(theta, t * nkv, hd, hd, nkv, pos_base, &ar.k)?;
	}
	gpu_scale_f64_inplace(attn_scale, t * qd, &ar.q)?;
	let max_bias = if sp.alibi { m.hp.alibi_bias } else { 0.0 };
	cached_gqa(&dec, ar, &ar.q, &ar.k, &ar.v, t, nqh, nkv, hd, kd, kd, max_bias, sp.bidir, &ar.attn)?;
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
		gpu_rmsnorm_f64(&ar.o, norm_of(m, l, Nk::PostAttn)?, &m.eps, t, ne, &ar.o)?;
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
/// `ar.attn_out`.
fn mla_attn_block(m: &Model, l: usize, sp: &Spec, h_in: &GpuBuffer, t: usize, ar: &Arena, dec: &DecCtx) -> Result<()> {
	let hp = &m.hp;
	let ne = hp.ne;
	let nqh = hp.dims[l].nqh;
	if nqh != 1 {
		return Err(anyhow!("{}: MLA absorbed path supports n_head == 1, got {nqh}", hp.arch));
	}
	let (qlr, kvlr, rot) = (hp.q_lora_rank, hp.kv_lora_rank, hp.n_rot);
	let (hdk, hdv) = (hp.head_k_mla, hp.head_v_mla);
	let nope = hdk - rot;
	let pos_base = dec.cached;
	let theta = &m.theta_full;
	blk_norm(m, sp, l, Nk::Input, t, ne, h_in, &ar.x)?;
	gpu_gemm_bt_f64(&ar.x, &m.stream(&layer_name(l, "self_attn.q_a_proj.weight"))?, t, qlr, ne, &ar.mqa)?;
	gpu_rmsnorm_f64(&ar.mqa, norm_of(m, l, Nk::QANorm)?, &m.eps, t, qlr, &ar.mqa)?;
	gpu_gemm_bt_f64(&ar.mqa, &m.stream(&layer_name(l, "self_attn.q_b_proj.weight"))?, t, nqh * hdk, qlr, &ar.mqb)?;
	// RoPE the tail rope sub-dim of every head in place (view at the nope offset).
	let qpe_view = ar.mqb.view(nope, t * nqh * hdk - nope);
	gpu_rope_partial_pos(theta, t * nqh, hdk, rot, nqh, pos_base, &qpe_view)?;
	gpu_slice_lead_into(&ar.mqb, t * nqh, hdk, nope, &ar.mqn)?;
	gpu_slice_cols(&ar.mqb, t * nqh, hdk, nope, rot, &ar.mqp)?;
	gpu_gemm_bt_f64(&ar.mqn, &m.stream(&layer_name(l, "self_attn.k_b_proj.weight"))?, t, kvlr, nope, &ar.mqx)?;
	gpu_concat_into(&ar.mqx, &ar.mqp, t, kvlr, rot, &ar.mqc)?;
	gpu_gemm_bt_f64(&ar.x, &m.stream(&layer_name(l, "self_attn.kv_a_proj.weight"))?, t, kvlr + rot, ne, &ar.mkv)?;
	gpu_slice_lead_into(&ar.mkv, t, kvlr + rot, kvlr, &ar.mkc)?;
	gpu_slice_cols(&ar.mkv, t, kvlr + rot, kvlr, rot, &ar.mkp)?;
	gpu_rmsnorm_f64(&ar.mkc, norm_of(m, l, Nk::KvANorm)?, &m.eps, t, kvlr, &ar.mkc)?;
	gpu_rope_partial_pos(theta, t, rot, rot, 1, pos_base, &ar.mkp)?;
	gpu_concat_into(&ar.mkc, &ar.mkp, t, kvlr, rot, &ar.mkk)?;
	gpu_scale_f64_inplace(&m.attn_scale_mla, t * (kvlr + rot), &ar.mqc)?;
	let kw = kvlr + rot;
	cached_mla(&dec, ar, &ar.mqc, &ar.mkk, &ar.mkc, t, nqh, kw, kvlr, &ar.mrw)?;
	gpu_gemm_bt_f64(&ar.mrw, &m.stream(&layer_name(l, "self_attn.v_b_proj.weight"))?, t, hdv, kvlr, &ar.mav)?;
	gpu_gemm_bt_f64(&ar.mav, &m.stream(&layer_name(l, "self_attn.o_proj.weight"))?, t, ne, nqh * hdv, &ar.o)?;
	gpu_add_into(&ar.o, h_in, t * ne, &ar.attn_out)?;
	return Ok(());
}

/// NeoX RoPE with optional LongRoPE per-pair frequency factors (minicpm3): the
/// `factors` path divides each pair's angle by `factors[i]`, the `None` path is
/// plain RoPE. Keeps [`layer_minicpm3`] agnostic to whether the arch ships factors.
fn rope_maybe_factors(
	theta: &GpuBuffer,
	factors: Option<&GpuBuffer>,
	rows: usize,
	head_dim: usize,
	rot: usize,
	heads: usize,
	buf: &GpuBuffer,
) -> Result<()> {
	match factors {
		Some(f) => gpu_rope_partial_factors(theta, rows, head_dim, rot, heads, f, buf)?,
		None => gpu_rope_partial(theta, rows, head_dim, rot, heads, buf)?,
	}
	return Ok(());
}

/// The minicpm3 decoder block (minicpm3.cpp:91-238): a naive (non-absorbed)
/// Multi-head Latent Attention with per-head RoPE'd query and one shared RoPE'd
/// key replicated across the query heads, LongRoPE per-pair frequency factors, and
/// the minicpm depth-scaled residual (both the attn and FFN outputs scaled by
/// `scale_depth/sqrt(n_layer)` via `m.res_scale`). The input embedding is pre-scaled
/// by `scale_embd` in the decode loop. Requires the zero-nope head geometry
/// (`n_embd_head_k == n_rot`, the fixture's shape); a nonzero nope split fails clean.
pub(super) fn layer_minicpm3(
	m: &Model,
	l: usize,
	sp: &Spec,
	h_in: &GpuBuffer,
	h_out: &GpuBuffer,
	t: usize,
	ar: &Arena,
	attn_scale: &GpuBuffer,
	dec: &DecCtx,
) -> Result<()> {
	let hp = &m.hp;
	let ne = hp.ne;
	let nqh = hp.dims[l].nqh;
	let (qlr, kvlr, rot) = (hp.q_lora_rank, hp.kv_lora_rank, hp.n_rot);
	let (hdk, hdv) = (hp.head_k_mla, hp.head_v_mla);
	if hdk != rot {
		return Err(anyhow!(
			"{}: minicpm3 nonzero-nope MLA (head_k={hdk}, n_rot={rot}) not supported",
			hp.arch
		));
	}
	let pos_base = dec.cached;
	let (kwid, vwid) = (nqh * hdk, nqh * hdv);
	let theta = &m.theta_full;
	let factors = m.rope_factors.as_ref();
	gpu_rmsnorm_f64(h_in, norm_of(m, l, Nk::Input)?, &m.eps, t, ne, &ar.x)?;
	gpu_gemm_bt_f64(&ar.x, &m.stream(&layer_name(l, "self_attn.q_a_proj.weight"))?, t, qlr, ne, &ar.mqa)?;
	gpu_rmsnorm_f64(&ar.mqa, norm_of(m, l, Nk::QANorm)?, &m.eps, t, qlr, &ar.mqa)?;
	gpu_gemm_bt_f64(&ar.mqa, &m.stream(&layer_name(l, "self_attn.q_b_proj.weight"))?, t, nqh * hdk, qlr, &ar.mqb)?;
	rope_maybe_factors_pos(theta, factors, t * nqh, hdk, rot, nqh, pos_base, &ar.mqb)?;
	gpu_gemm_bt_f64(&ar.x, &m.stream(&layer_name(l, "self_attn.kv_a_proj.weight"))?, t, kvlr + rot, ne, &ar.mkv)?;
	gpu_slice_lead_into(&ar.mkv, t, kvlr + rot, kvlr, &ar.mkc)?;
	gpu_slice_cols(&ar.mkv, t, kvlr + rot, kvlr, rot, &ar.mkp)?;
	gpu_rmsnorm_f64(&ar.mkc, norm_of(m, l, Nk::KvANorm)?, &m.eps, t, kvlr, &ar.mkc)?;
	rope_maybe_factors_pos(theta, factors, t, rot, rot, 1, pos_base, &ar.mkp)?;
	gpu_gemm_bt_f64(&ar.mkc, &m.stream(&layer_name(l, "self_attn.kv_b_proj.weight"))?, t, nqh * hdv, kvlr, &ar.v)?;
	gpu_copy_into(&ar.mkp, t * rot, &ar.k)?;
	for h in 1..nqh {
		gpu_concat_into(&ar.k, &ar.mkp, t, h * rot, rot, &ar.mkk)?;
		gpu_copy_into(&ar.mkk, t * (h + 1) * rot, &ar.k)?;
	}
	gpu_scale_f64_inplace(attn_scale, t * nqh * hdk, &ar.mqb)?;
	cached_gqa(&dec, ar, &ar.mqb, &ar.k, &ar.v, t, nqh, nqh, hdk, kwid, vwid, 0.0, false, &ar.attn)?;
	gpu_gemm_bt_f64(&ar.attn, &m.stream(&layer_name(l, "self_attn.o_proj.weight"))?, t, ne, nqh * hdv, &ar.o)?;
	gpu_scale_f64_inplace(&m.res_scale, t * ne, &ar.o)?;
	gpu_add_into(&ar.o, h_in, t * ne, &ar.attn_out)?;
	gpu_rmsnorm_f64(&ar.attn_out, norm_of(m, l, Nk::PreFf)?, &m.eps, t, ne, &ar.cms)?;
	ffn(m, l, sp, t, ar, &ar.cms)?;
	gpu_scale_f64_inplace(&m.res_scale, t * ne, &ar.mlp0)?;
	gpu_add_into(&ar.mlp0, &ar.attn_out, t * ne, h_out)?;
	return Ok(());
}

/// Parameterized dense-attention decoder block: attention, residual, FFN
/// (gated SwiGLU/GeGLU or sequential GELU/ReLU^2), residual.
pub(super) fn layer_spec(
	m: &Model,
	l: usize,
	sp: &Spec,
	h_in: &GpuBuffer,
	h_out: &GpuBuffer,
	t: usize,
	ar: &Arena,
	attn_scale: &GpuBuffer,
	dec: &DecCtx,
) -> Result<()> {
	let ne = m.hp.ne;
	attn_block(m, l, sp, h_in, t, ar, attn_scale, dec)?;
	let ffn_in = if sp.parallel {
		&ar.x
	} else if !sp.pre_norm {
		&ar.attn_out
	} else {
		blk_norm(m, sp, l, Nk::PreFf, t, ne, &ar.attn_out, &ar.cms)?;
		&ar.cms
	};
	ffn(m, l, sp, t, ar, ffn_in)?;
	if sp.post_ffn {
		gpu_rmsnorm_f64(&ar.mlp0, norm_of(m, l, Nk::Pfw)?, &m.eps, t, ne, &ar.mlp0)?;
	}
	if sp.residual_scale {
		gpu_scale_f64_inplace(&m.res_scale, t * ne, &ar.mlp0)?;
	}
	gpu_add_into(&ar.mlp0, &ar.attn_out, t * ne, h_out)?;
	Ok(())
}

/// The talkie decoder block (talkie.cpp:64-129). Every block norm is
/// non-parametric RMS; the attention applies RoPE first, then an asymmetric
/// post-rope qk-norm (Q: non-parametric RMS over head_dim then a per-head scalar
/// gain; K: non-parametric RMS, no gain); the FFN is a non-parametric-normed
/// SwiGLU; and the frozen non-parametric-normed initial embedding (`ar.embd_skip`,
/// stashed once by the decode loop) is added to every layer output scaled by that
/// layer's `layer_output_scale` scalar.
pub(super) fn layer_talkie(
	m: &Model,
	l: usize,
	sp: &Spec,
	h_in: &GpuBuffer,
	h_out: &GpuBuffer,
	t: usize,
	ar: &Arena,
	attn_scale: &GpuBuffer,
	dec: &DecCtx,
) -> Result<()> {
	let hp = &m.hp;
	let ne = hp.ne;
	let d = &hp.dims[l];
	let (nqh, hd, nkv) = (d.nqh, d.hd, d.nkv);
	let (qd, kd) = (nqh * hd, nkv * hd);
	let pos_base = dec.cached;
	gpu_rmsnorm_f64_nogamma(h_in, &m.eps, t, ne, &ar.x)?;
	gpu_gemm_bt_f64(&ar.x, &m.stream(&layer_name(l, "self_attn.q_proj.weight"))?, t, qd, ne, &ar.q)?;
	gpu_gemm_bt_f64(&ar.x, &m.stream(&layer_name(l, "self_attn.k_proj.weight"))?, t, kd, ne, &ar.k)?;
	gpu_gemm_bt_f64(&ar.x, &m.stream(&layer_name(l, "self_attn.v_proj.weight"))?, t, kd, ne, &ar.v)?;
	gpu_rope_partial_pos(&m.theta_full, t * nqh, hd, hd, nqh, pos_base, &ar.q)?;
	gpu_rope_partial_pos(&m.theta_full, t * nkv, hd, hd, nkv, pos_base, &ar.k)?;
	gpu_rmsnorm_f64_nogamma(&ar.q, &m.eps, t * nqh, hd, &ar.q)?;
	let gain = m.q_headscale[l]
		.as_ref()
		.ok_or_else(|| anyhow!("{}: layer {l} missing per-head q gain", hp.arch))?;
	gpu_broadcast_mul(&ar.q, gain, t * qd, qd, &ar.q)?;
	gpu_rmsnorm_f64_nogamma(&ar.k, &m.eps, t * nkv, hd, &ar.k)?;
	gpu_scale_f64_inplace(attn_scale, t * qd, &ar.q)?;
	cached_gqa(&dec, ar, &ar.q, &ar.k, &ar.v, t, nqh, nkv, hd, kd, kd, 0.0, false, &ar.attn)?;
	gpu_gemm_bt_f64(&ar.attn, &m.stream(&layer_name(l, "self_attn.o_proj.weight"))?, t, ne, qd, &ar.o)?;
	gpu_add_into(&ar.o, h_in, t * ne, &ar.attn_out)?;
	gpu_rmsnorm_f64_nogamma(&ar.attn_out, &m.eps, t, ne, &ar.cms)?;
	ffn(m, l, sp, t, ar, &ar.cms)?;
	gpu_add_into(&ar.mlp0, &ar.attn_out, t * ne, &ar.o)?;
	gpu_copy_into(&ar.embd_skip, t * ne, &ar.cms)?;
	gpu_scale_f64_inplace(&m.ls_dev[l], t * ne, &ar.cms)?;
	gpu_add_into(&ar.cms, &ar.o, t * ne, h_out)?;
	return Ok(());
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
	dec: &DecCtx,
) -> Result<()> {
	let ne = m.hp.ne;
	attn_block(m, l, sp, h_in, t, ar, attn_scale, dec)?;
	blk_norm(m, sp, l, Nk::PreFf, t, ne, &ar.attn_out, &ar.cms)?;
	moe_core(m, l, &ar.cms, t, ar, false, None)?;
	if sp.residual_scale {
		gpu_scale_f64_inplace(&m.res_scale, t * ne, &ar.mlp0)?;
	}
	gpu_add_into(&ar.mlp0, &ar.attn_out, t * ne, h_out)?;
	Ok(())
}

/// Linear-attention / state-space decoder block for the recurrent families
/// (Mamba, RWKV, gated delta-net hybrids). Composes the sequence mixer as a
/// gated linear recurrence over the sequence via [`gpu_scan_linear_recurrence`]
/// (the diagonal decay scan the SSM/WKV/delta families reduce to at decode) over
/// the SiLU-gated K and the V projection, then the SiLU SwiGLU FFN. Structural
/// composition of the `build_*` graph; per-family decay parameterization is
/// refined per arch. The Q projection is computed but not consumed by the scan.
pub(super) fn layer_recurrent(
	m: &Model,
	l: usize,
	h_in: &GpuBuffer,
	h_out: &GpuBuffer,
	t: usize,
	ar: &Arena,
	_attn_scale: &GpuBuffer,
	dec: &DecCtx,
) -> Result<()> {
	let hp = &m.hp;
	let ne = hp.ne;
	let d = &hp.dims[l];
	let nqh = d.nqh;
	let kd = d.nkv * d.hd;
	let qd = nqh * d.hd;
	gpu_rmsnorm_f64(h_in, norm_of(m, l, Nk::Input)?, &m.eps, t, ne, &ar.x)?;
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
	gpu_scan_linear_recurrence(&ar.k, &ar.v, t, kd, &ar.attn, rec_of(dec)?)?;
	gpu_gemm_bt_f64(
		&ar.attn,
		&m.stream(&layer_name(l, "self_attn.o_proj.weight"))?,
		t,
		ne,
		kd,
		&ar.o,
	)?;
	gpu_add_into(&ar.o, h_in, t * ne, &ar.attn_out)?;
	gpu_rmsnorm_f64(&ar.attn_out, norm_of(m, l, Nk::PreFf)?, &m.eps, t, ne, &ar.cms)?;
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
	dec: &DecCtx,
) -> Result<()> {
	mamba1_mix(m, l, h_in, &ar.o, t, ar, dec)?;
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
	dec: &DecCtx,
) -> Result<()> {
	let hp = &m.hp;
	let arch = hp.arch.as_str();
	let ne = hp.ne;
	let di = hp.ssm_d_inner;
	let ds = hp.ssm_d_state;
	let dr = hp.ssm_dt_rank;
	let dc = hp.ssm_d_conv;
	let dbw = dr + 2 * ds;
	let (cin, cout) = conv_io(dec, 0)?;
	if hp.ssm_n_group != 1 {
		return Err(anyhow!(
			"{arch}: mamba-1 block expects a single selective group, got n_group={}",
			hp.ssm_n_group
		));
	}

	gpu_rmsnorm_f64(h_in, norm_of(m, l, Nk::Input)?, &m.eps, t, ne, &ar.x)?;

	let w_in = m.stream(&layer_name(l, "self_attn.ssm_in.weight"))?;
	gpu_gemm_bt_f64(&ar.x, &w_in.view(0, di * ne), t, di, ne, &ar.ss_x)?;
	gpu_gemm_bt_f64(&ar.x, &w_in.view(di * ne, di * ne), t, di, ne, &ar.ss_z)?;

	let conv_w = ssm_of(&m.ssm_conv_w, l, arch, "ssm_conv1d.weight")?;
	let conv_b = ssm_of(&m.ssm_conv_b, l, arch, "ssm_conv1d.bias")?;
	gpu_ssm_conv_causal_silu(&ar.ss_x, conv_w, Some(conv_b), t, di, dc, &ar.ss_xc, cin, cout)?;

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
	gpu_ssm_scan_mamba1(&ar.ss_xc, &ar.ss_dt, a, &ar.ss_bb, &ar.ss_cc, d, t, di, ds, &ar.ss_y, rec_of(dec)?)?;

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
	dec: &DecCtx,
) -> Result<()> {
	mamba2_mix(m, l, h_in, &ar.o, t, ar, dec)?;
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
	dec: &DecCtx,
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
	let (cin, cout) = conv_io(dec, 0)?;
	if nh == 0 || di % nh != 0 {
		return Err(anyhow!("{arch}: mamba-2 d_inner={di} not divisible by n_head={nh}"));
	}
	if ng == 0 || di % ng != 0 {
		return Err(anyhow!("{arch}: mamba-2 d_inner={di} not divisible by n_group={ng}"));
	}

	gpu_rmsnorm_f64(h_in, norm_of(m, l, Nk::Input)?, &m.eps, t, ne, &ar.x)?;

	let w_in = m.stream(&layer_name(l, "self_attn.ssm_in.weight"))?;
	gpu_gemm_bt_f64(&ar.x, &w_in.view(0, di * ne), t, di, ne, &ar.ss_z)?;
	gpu_gemm_bt_f64(&ar.x, &w_in.view(di * ne, conv_dim * ne), t, conv_dim, ne, &ar.ss_xbc)?;
	gpu_gemm_bt_f64(&ar.x, &w_in.view((di + conv_dim) * ne, nh * ne), t, nh, ne, &ar.ss_dtlr)?;

	let conv_w = ssm_of(&m.ssm_conv_w, l, arch, "ssm_conv1d.weight")?;
	let conv_b = ssm_of(&m.ssm_conv_b, l, arch, "ssm_conv1d.bias")?;
	gpu_ssm_conv_causal_silu(&ar.ss_xbc, conv_w, Some(conv_b), t, conv_dim, dc, &ar.ss_xbcc, cin, cout)?;

	let dt_b = ssm_of(&m.ssm_dt_b, l, arch, "ssm_dt.bias")?;
	gpu_bias_add(&ar.ss_dtlr, dt_b, t, nh, &ar.ss_dtlr)?;

	let a = ssm_of(&m.ssm_a, l, arch, "ssm_a")?;
	let d = ssm_of(&m.ssm_d, l, arch, "ssm_d")?;
	gpu_ssm_scan_mamba2(&ar.ss_xbcc, &ar.ss_dtlr, a, d, t, di, ds, nh, ng, conv_dim, &ar.ss_y, rec_of(dec)?)?;

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
	dec: &DecCtx,
) -> Result<()> {
	let hp = &m.hp;
	let arch = hp.arch.as_str();
	let ne = hp.ne;
	let di = hp.ssm_d_inner;
	let ds = hp.ssm_d_state;
	let nh = hp.ssm_dt_rank;
	let dc = hp.ssm_d_conv;
	let dtd = hp.ssm_dt_dim;
	let (cin, cout) = conv_io(dec, 0)?;
	if nh == 0 || di % nh != 0 {
		return Err(anyhow!("{arch}: plamo2 d_inner={di} not divisible by n_head={nh}"));
	}
	let head_dim = di / nh;
	let bcd = dtd + 2 * ds;
	let conv_dim = di + 2 * ds;

	gpu_rmsnorm_f64(h_in, norm_of(m, l, Nk::Input)?, &m.eps, t, ne, &ar.x)?;

	let w_in = m.stream(&layer_name(l, "self_attn.ssm_in.weight"))?;
	gpu_gemm_bt_f64(&ar.x, &w_in, t, 2 * di, ne, &ar.ss_zx)?;
	deinterleave_heads(&ar.ss_zx, t, head_dim, nh, head_dim, &ar.ss_x, &ar.ss_dt, &ar.ss_y)?;
	deinterleave_heads(&ar.ss_zx, t, head_dim, nh, 0, &ar.ss_z, &ar.ss_dt, &ar.ss_y)?;

	let conv_w = ssm_of(&m.ssm_conv_w, l, arch, "ssm_conv1d.weight")?;
	let conv_b = ssm_of(&m.ssm_conv_b, l, arch, "ssm_conv1d.bias")?;
	gpu_ssm_conv_causal_silu(&ar.ss_x, conv_w, Some(conv_b), t, di, dc, &ar.ss_xc, cin, cout)?;

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
	gpu_ssm_scan_mamba2(&ar.ss_xbc, &dt, a, d, t, di, ds, nh, 1, conv_dim, &ar.ss_y, rec_of(dec)?)?;

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
	dec: &DecCtx,
) -> Result<()> {
	let hp = &m.hp;
	let ne = hp.ne;
	let d = &hp.dims[l];
	let nqh = d.nqh;
	let (hd, nkv, qd, kd) = (d.hd, d.nkv, nqh * d.hd, d.nkv * d.hd);
	let pos_base = dec.cached;
	gpu_rmsnorm_f64(h_in, norm_of(m, l, Nk::Input)?, &m.eps, t, ne, &ar.x)?;
	gpu_gemm_bt_f64(&ar.x, &m.stream(&layer_name(l, "self_attn.q_proj.weight"))?, t, qd, ne, &ar.q)?;
	gpu_gemm_bt_f64(&ar.x, &m.stream(&layer_name(l, "self_attn.k_proj.weight"))?, t, kd, ne, &ar.k)?;
	gpu_gemm_bt_f64(&ar.x, &m.stream(&layer_name(l, "self_attn.v_proj.weight"))?, t, kd, ne, &ar.v)?;
	gpu_rmsnorm_f64_nogamma(&ar.q, &m.eps, t * nqh, hd, &ar.q)?;
	gpu_broadcast_mul(&ar.q, norm_of(m, l, Nk::QNorm)?, t * qd, qd, &ar.q)?;
	gpu_rmsnorm_f64_nogamma(&ar.k, &m.eps, t * nkv, hd, &ar.k)?;
	gpu_broadcast_mul(&ar.k, norm_of(m, l, Nk::KNorm)?, t * kd, kd, &ar.k)?;
	gpu_rope_partial_pos(&m.theta_full, t * nqh, hd, hd, nqh, pos_base, &ar.q)?;
	gpu_rope_partial_pos(&m.theta_full, t * nkv, hd, hd, nkv, pos_base, &ar.k)?;
	gpu_scale_f64_inplace(attn_scale, t * qd, &ar.q)?;
	cached_gqa(&dec, ar, &ar.q, &ar.k, &ar.v, t, nqh, nkv, hd, kd, kd, 0.0, false, &ar.attn)?;
	gpu_gemm_bt_f64(&ar.attn, &m.stream(&layer_name(l, "self_attn.o_proj.weight"))?, t, ne, qd, out)?;
	return Ok(());
}

/// True if layer `l` carries a recurrent SSM mixer (its `ssm_in` projection is
/// present) rather than attention: the honest per-layer interleave signal, the
/// same discriminator `llama.cpp` derives from `n_head_kv(l)==0`.
pub(super) fn layer_is_recur(m: &Model, l: usize) -> bool {
	return m.big.contains_key(&layer_name(l, "self_attn.ssm_in.weight"));
}

/// True if layer `l` is a gated-delta / KDA recurrent layer, keyed on the presence
/// of its causal-conv signature tensor (the mixed `ssm_conv1d` for GDA, the `q`
/// short-conv for KDA) — the honest per-layer interleave discriminator, absent on
/// the arch's full-attention layers.
pub(super) fn layer_is_delta(m: &Model, l: usize) -> bool {
	return m.big.contains_key(&layer_name(l, "self_attn.ssm_conv1d.weight"))
		|| m.big.contains_key(&layer_name(l, "self_attn.q_conv.weight"));
}

/// The gated-delta head geometry `(d, hk, hv, key_dim, value_dim, conv_dim)`. For
/// GDA arches `d = ssm.state_size`, `hk = ssm.group_count`, `hv =
/// ssm.time_step_rank`; for KDA (kimi) `d = kda.head_dim` and both head counts are
/// `kda_n_head` (separate per-projection convs, so `conv_dim = d_inner`).
pub(super) fn delta_dims(m: &Model) -> (usize, usize, usize, usize, usize, usize) {
	let hp = &m.hp;
	if hp.kda_head_dim > 0 {
		let (d, h) = (hp.kda_head_dim, hp.kda_n_head);
		let di = d * h;
		return (d, h, h, di, di, di);
	}
	let (d, hk, hv) = (hp.ssm_d_state, hp.ssm_n_group, hp.ssm_dt_rank);
	let (key_dim, value_dim) = (hk * d, hv * d);
	return (d, hk, hv, key_dim, value_dim, 2 * key_dim + value_dim);
}

/// The gated delta-net (GDA) recurrent mixer (qwen3.5/next/moe
/// `build_layer_attn_linear`), writing the `ssm_out`-projected mixer output to
/// `out` without the block residual. Pre-norm, q|k|v|z projections (fused `wqkv`
/// + `wqkv_gate`), the fused-or-separate beta/alpha, causal short conv + SiLU,
/// per-head L2-norm of q/k, the delta-rule scan (per-head scalar decay), and the
/// SiLU(z)-gated `ssm_norm` output. All scratch is arena-resident.
fn gated_delta_mix(m: &Model, l: usize, h_in: &GpuBuffer, out: &GpuBuffer, t: usize, ar: &Arena, dec: &DecCtx) -> Result<()> {
	let hp = &m.hp;
	let arch = hp.arch.as_str();
	let ne = hp.ne;
	let (d, hk, hv, key_dim, value_dim, conv_dim) = delta_dims(m);
	let dc = hp.ssm_d_conv;
	let (cin, cout) = conv_io(dec, 0)?;
	gpu_rmsnorm_f64(h_in, norm_of(m, l, Nk::Input)?, &m.eps, t, ne, &ar.x)?;
	gpu_gemm_bt_f64(&ar.x, &m.stream(&layer_name(l, "self_attn.qkv_proj.weight"))?, t, conv_dim, ne, &ar.d_qkv)?;
	gpu_gemm_bt_f64(&ar.x, &m.stream(&layer_name(l, "self_attn.z_gate.weight"))?, t, value_dim, ne, &ar.d_z)?;
	if m.big.contains_key(&layer_name(l, "self_attn.ssm_ba.weight")) {
		gpu_gemm_bt_f64(&ar.x, &m.stream(&layer_name(l, "self_attn.ssm_ba.weight"))?, t, 2 * hv, ne, &ar.d_o)?;
		deinterleave_heads(&ar.d_o, t, 1, hv, 0, &ar.d_bt, &ar.d_q, &ar.d_k)?;
		deinterleave_heads(&ar.d_o, t, 1, hv, 1, &ar.d_g, &ar.d_q, &ar.d_k)?;
	} else {
		gpu_gemm_bt_f64(&ar.x, &m.stream(&layer_name(l, "self_attn.ssm_beta.weight"))?, t, hv, ne, &ar.d_bt)?;
		gpu_gemm_bt_f64(&ar.x, &m.stream(&layer_name(l, "self_attn.ssm_alpha.weight"))?, t, hv, ne, &ar.d_g)?;
	}
	gpu_sigmoid_into(&ar.d_bt, t * hv, &ar.d_bt)?;
	gpu_bias_add(&ar.d_g, ssm_of(&m.ssm_dt_b, l, arch, "ssm_dt.bias")?, t, hv, &ar.d_g)?;
	gpu_softplus(&ar.d_g, t * hv, &ar.d_g)?;
	let conv_w = ssm_of(&m.ssm_conv_w, l, arch, "ssm_conv1d.weight")?;
	gpu_ssm_conv_causal_silu(&ar.d_qkv, conv_w, None, t, conv_dim, dc, &ar.d_cv, cin, cout)?;
	gpu_slice_cols(&ar.d_cv, t, conv_dim, 0, key_dim, &ar.d_q)?;
	gpu_slice_cols(&ar.d_cv, t, conv_dim, key_dim, key_dim, &ar.d_k)?;
	gpu_slice_cols(&ar.d_cv, t, conv_dim, 2 * key_dim, value_dim, &ar.d_v)?;
	gpu_l2norm_rows(&ar.d_q, &m.eps, t * hk, d, &ar.d_q)?;
	gpu_l2norm_rows(&ar.d_k, &m.eps, t * hk, d, &ar.d_k)?;
	let a = ssm_of(&m.ssm_a, l, arch, "ssm_a")?;
	let scale = 1.0 / (d as f64).sqrt();
	gpu_gated_delta_scan(&ar.d_q, &ar.d_k, &ar.d_v, &ar.d_g, &ar.d_bt, a, &ar.d_o, t, hv, d, false, scale, rec_of(dec)?)?;
	gpu_rmsnorm_f64(&ar.d_o, ssm_of(&m.ssm_norm, l, arch, "ssm_norm.weight")?, &m.eps, t * hv, d, &ar.d_o)?;
	gpu_silu_into(&ar.d_z, t * value_dim, &ar.d_z)?;
	gpu_mul_inplace(&ar.d_z, t * value_dim, &ar.d_o)?;
	gpu_gemm_bt_f64(&ar.d_o, &m.stream(&layer_name(l, "self_attn.ssm_out.weight"))?, t, ne, value_dim, out)?;
	return Ok(());
}

/// The Kimi Delta-Attention (KDA) recurrent mixer (kimi-linear graph :288-372),
/// writing the `o_proj`-projected output to `out` without the block residual.
/// Separate q/k/v projections each with its own causal short conv + SiLU, per-head
/// L2-norm of q/k, a per-CHANNEL log-decay LoRA (`f_b(f_a(x))`), the delta-rule
/// scan (per-channel decay), and a sigmoid(`g_b(g_a(x))`)-gated `ssm_norm` output.
fn kda_mix(m: &Model, l: usize, h_in: &GpuBuffer, out: &GpuBuffer, t: usize, ar: &Arena, dec: &DecCtx) -> Result<()> {
	let hp = &m.hp;
	let arch = hp.arch.as_str();
	let ne = hp.ne;
	let (d, h) = (hp.kda_head_dim, hp.kda_n_head);
	let di = d * h;
	let dc = hp.ssm_d_conv;
	let (qin, qout) = conv_io(dec, 0)?;
	let (kin, kout) = conv_io(dec, 1)?;
	let (vin, vout) = conv_io(dec, 2)?;
	gpu_rmsnorm_f64(h_in, norm_of(m, l, Nk::Input)?, &m.eps, t, ne, &ar.x)?;
	let qcw = ssm_of(&m.ssm_q_conv_w, l, arch, "q_conv.weight")?;
	gpu_gemm_bt_f64(&ar.x, &m.stream(&layer_name(l, "self_attn.q_proj.weight"))?, t, di, ne, &ar.d_qkv)?;
	gpu_ssm_conv_causal_silu(&ar.d_qkv, qcw, None, t, di, dc, &ar.d_q, qin, qout)?;
	let kcw = ssm_of(&m.ssm_k_conv_w, l, arch, "k_conv.weight")?;
	gpu_gemm_bt_f64(&ar.x, &m.stream(&layer_name(l, "self_attn.k_proj.weight"))?, t, di, ne, &ar.d_qkv)?;
	gpu_ssm_conv_causal_silu(&ar.d_qkv, kcw, None, t, di, dc, &ar.d_k, kin, kout)?;
	let vcw = ssm_of(&m.ssm_v_conv_w, l, arch, "v_conv.weight")?;
	gpu_gemm_bt_f64(&ar.x, &m.stream(&layer_name(l, "self_attn.v_proj.weight"))?, t, di, ne, &ar.d_qkv)?;
	gpu_ssm_conv_causal_silu(&ar.d_qkv, vcw, None, t, di, dc, &ar.d_v, vin, vout)?;
	gpu_l2norm_rows(&ar.d_q, &m.eps, t * h, d, &ar.d_q)?;
	gpu_l2norm_rows(&ar.d_k, &m.eps, t * h, d, &ar.d_k)?;
	gpu_gemm_bt_f64(&ar.x, &m.stream(&layer_name(l, "self_attn.f_a.weight"))?, t, d, ne, &ar.d_z)?;
	gpu_gemm_bt_f64(&ar.d_z, &m.stream(&layer_name(l, "self_attn.f_b.weight"))?, t, di, d, &ar.d_g)?;
	gpu_bias_add(&ar.d_g, ssm_of(&m.ssm_dt_b, l, arch, "ssm_dt.bias")?, t, di, &ar.d_g)?;
	gpu_softplus(&ar.d_g, t * di, &ar.d_g)?;
	gpu_gemm_bt_f64(&ar.x, &m.stream(&layer_name(l, "self_attn.ssm_beta.weight"))?, t, h, ne, &ar.d_bt)?;
	gpu_sigmoid_into(&ar.d_bt, t * h, &ar.d_bt)?;
	let a = ssm_of(&m.ssm_a, l, arch, "ssm_a")?;
	let scale = 1.0 / (d as f64).sqrt();
	gpu_gated_delta_scan(&ar.d_q, &ar.d_k, &ar.d_v, &ar.d_g, &ar.d_bt, a, &ar.d_o, t, h, d, true, scale, rec_of(dec)?)?;
	gpu_gemm_bt_f64(&ar.x, &m.stream(&layer_name(l, "self_attn.g_a.weight"))?, t, d, ne, &ar.d_z)?;
	gpu_gemm_bt_f64(&ar.d_z, &m.stream(&layer_name(l, "self_attn.g_b.weight"))?, t, di, d, &ar.d_qkv)?;
	gpu_sigmoid_into(&ar.d_qkv, t * di, &ar.d_qkv)?;
	gpu_rmsnorm_f64(&ar.d_o, ssm_of(&m.ssm_norm, l, arch, "ssm_norm.weight")?, &m.eps, t * h, d, &ar.d_o)?;
	gpu_mul_inplace(&ar.d_qkv, t * di, &ar.d_o)?;
	gpu_gemm_bt_f64(&ar.d_o, &m.stream(&layer_name(l, "self_attn.o_proj.weight"))?, t, ne, di, out)?;
	return Ok(());
}

/// The gated delta-net full-attention companion (qwen3.5/next `build_layer_attn`),
/// writing the `o_proj`-projected output to `out` without the block residual. A
/// fused Q+gate projection (per-head `[Q | gate]`), per-head Q/K RMSNorm, NEOX
/// RoPE (qwen3.5's MRoPE reduces to this for text positions), scaled GQA, then a
/// sigmoid(gate) elementwise gate before `o_proj`.
fn delta_full_attn(
	m: &Model,
	l: usize,
	h_in: &GpuBuffer,
	out: &GpuBuffer,
	t: usize,
	ar: &Arena,
	attn_scale: &GpuBuffer,
	dec: &DecCtx,
) -> Result<()> {
	let hp = &m.hp;
	let ne = hp.ne;
	let dd = &hp.dims[l];
	let (nqh, hd, nkv) = (dd.nqh, dd.hd, dd.nkv);
	let (qd, kd) = (nqh * hd, nkv * hd);
	let pos_base = dec.cached;
	gpu_rmsnorm_f64(h_in, norm_of(m, l, Nk::Input)?, &m.eps, t, ne, &ar.x)?;
	gpu_gemm_bt_f64(&ar.x, &m.stream(&layer_name(l, "self_attn.q_proj.weight"))?, t, 2 * qd, ne, &ar.d_qkv)?;
	deinterleave_heads(&ar.d_qkv, t, hd, nqh, 0, &ar.q, &ar.d_q, &ar.d_k)?;
	deinterleave_heads(&ar.d_qkv, t, hd, nqh, hd, &ar.d_z, &ar.d_q, &ar.d_k)?;
	gpu_gemm_bt_f64(&ar.x, &m.stream(&layer_name(l, "self_attn.k_proj.weight"))?, t, kd, ne, &ar.k)?;
	gpu_gemm_bt_f64(&ar.x, &m.stream(&layer_name(l, "self_attn.v_proj.weight"))?, t, kd, ne, &ar.v)?;
	gpu_rmsnorm_f64(&ar.q, norm_of(m, l, Nk::QNorm)?, &m.eps, t * nqh, hd, &ar.q)?;
	gpu_rmsnorm_f64(&ar.k, norm_of(m, l, Nk::KNorm)?, &m.eps, t * nkv, hd, &ar.k)?;
	gpu_rope_partial_pos(&m.theta_full, t * nqh, hd, hd, nqh, pos_base, &ar.q)?;
	gpu_rope_partial_pos(&m.theta_full, t * nkv, hd, hd, nkv, pos_base, &ar.k)?;
	gpu_scale_f64_inplace(attn_scale, t * qd, &ar.q)?;
	cached_gqa(&dec, ar, &ar.q, &ar.k, &ar.v, t, nqh, nkv, hd, kd, kd, 0.0, false, &ar.attn)?;
	gpu_sigmoid_into(&ar.d_z, t * qd, &ar.d_z)?;
	gpu_mul_inplace(&ar.d_z, t * qd, &ar.attn)?;
	gpu_gemm_bt_f64(&ar.attn, &m.stream(&layer_name(l, "self_attn.o_proj.weight"))?, t, ne, qd, out)?;
	return Ok(());
}

/// The kimi-linear full-attention companion: absorbed MLA (deepseek2-style) with
/// NO RoPE and NO q_a compression (kimi-linear.cpp:374-472, `wk_b`/`wv_b` present).
/// Q rides `wq` directly, splits into a nope part (absorbed into the kv latent by
/// `k_b`) and a pe part carried unrotated; K/V ride the `kv_a` latent (RMSNorm'd)
/// plus the unrotated k_pe. MQA over `kv_lora+pe` keys and `kv_lora` values, then
/// `v_b` decompress and `o_proj`. Reuses the MLA arena scratch.
fn kimi_mla_attn(m: &Model, l: usize, h_in: &GpuBuffer, out: &GpuBuffer, t: usize, ar: &Arena, dec: &DecCtx) -> Result<()> {
	let hp = &m.hp;
	let ne = hp.ne;
	let nqh = hp.dims[l].nqh;
	if nqh != 1 {
		return Err(anyhow!("{}: kimi MLA supports n_head == 1, got {nqh}", hp.arch));
	}
	let (kvlr, rot) = (hp.kv_lora_rank, hp.n_rot);
	let (hdk, hdv) = (hp.head_k_mla, hp.head_v_mla);
	let nope = hdk - rot;
	gpu_rmsnorm_f64(h_in, norm_of(m, l, Nk::Input)?, &m.eps, t, ne, &ar.x)?;
	gpu_gemm_bt_f64(&ar.x, &m.stream(&layer_name(l, "self_attn.q_proj.weight"))?, t, nqh * hdk, ne, &ar.mqb)?;
	gpu_slice_lead_into(&ar.mqb, t * nqh, hdk, nope, &ar.mqn)?;
	gpu_slice_cols(&ar.mqb, t * nqh, hdk, nope, rot, &ar.mqp)?;
	gpu_gemm_bt_f64(&ar.mqn, &m.stream(&layer_name(l, "self_attn.k_b_proj.weight"))?, t, kvlr, nope, &ar.mqx)?;
	gpu_concat_into(&ar.mqx, &ar.mqp, t, kvlr, rot, &ar.mqc)?;
	gpu_gemm_bt_f64(&ar.x, &m.stream(&layer_name(l, "self_attn.kv_a_proj.weight"))?, t, kvlr + rot, ne, &ar.mkv)?;
	gpu_slice_lead_into(&ar.mkv, t, kvlr + rot, kvlr, &ar.mkc)?;
	gpu_slice_cols(&ar.mkv, t, kvlr + rot, kvlr, rot, &ar.mkp)?;
	gpu_rmsnorm_f64(&ar.mkc, norm_of(m, l, Nk::KvANorm)?, &m.eps, t, kvlr, &ar.mkc)?;
	gpu_concat_into(&ar.mkc, &ar.mkp, t, kvlr, rot, &ar.mkk)?;
	gpu_scale_f64_inplace(&m.attn_scale_mla, t * (kvlr + rot), &ar.mqc)?;
	let kw = kvlr + rot;
	cached_mla(&dec, ar, &ar.mqc, &ar.mkk, &ar.mkc, t, nqh, kw, kvlr, &ar.mrw)?;
	gpu_gemm_bt_f64(&ar.mrw, &m.stream(&layer_name(l, "self_attn.v_b_proj.weight"))?, t, hdv, kvlr, &ar.mav)?;
	gpu_gemm_bt_f64(&ar.mav, &m.stream(&layer_name(l, "self_attn.o_proj.weight"))?, t, ne, nqh * hdv, out)?;
	return Ok(());
}

/// The routed mixture-of-experts core shared by [`layer_moe`] and the delta-net
/// FFN: gate logits, `softmax` or `sigmoid` gating, an optional selection bias
/// (`exp_probs_b`, added for top-k selection only), renormalized top-`used`
/// weights, and the per-expert SwiGLU compose. Writes the routed output to
/// `ar.mlp0`. The router/expert loop is host-driven (one D2H per expert), matching
/// the existing MoE serialization.
fn moe_core(
	m: &Model,
	l: usize,
	cms: &GpuBuffer,
	t: usize,
	ar: &Arena,
	sigmoid: bool,
	bias: Option<&GpuBuffer>,
) -> Result<()> {
	let hp = &m.hp;
	let (ne, nffe, nexp, used) = (hp.ne, hp.nffe, hp.nexp, hp.used);
	let gate_w = m.stream(&layer_name(l, "router.proj.weight"))?;
	let logits = GpuBuffer::alloc_ty(t * nexp, super::super::FWD_DT)?;
	gpu_gemm_bt_f64(cms, &gate_w, t, nexp, ne, &logits)?;
	let mut lh = vec![0.0f64; t * nexp];
	logits.download_host(&mut lh)?;
	let mut cms_host = vec![0.0f64; t * ne];
	cms.download_host(&mut cms_host)?;
	let mut bias_h = vec![0.0f64; nexp];
	if let Some(b) = bias {
		b.download_host(&mut bias_h)?;
	}
	gpu_core::hip::device_synchronize()?;
	let mut e2p: BTreeMap<usize, Vec<(usize, f64)>> = BTreeMap::new();
	for p in 0..t {
		let mut probs = lh[p * nexp..(p + 1) * nexp].to_vec();
		if sigmoid {
			for v in probs.iter_mut() {
				*v = 1.0 / (1.0 + (-*v).exp());
			}
		} else {
			softmax(&mut probs);
		}
		let mut idx: Vec<usize> = (0..nexp).collect();
		idx.sort_by(|a, b| {
			(probs[*b] + bias_h[*b])
				.partial_cmp(&(probs[*a] + bias_h[*a]))
				.unwrap_or(cmp::Ordering::Equal)
		});
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
		ar.moe_dv.download_host(&mut dv[..np * ne])?;
		gpu_core::hip::device_synchronize()?;
		for (i, &(p, w)) in poslist.iter().enumerate() {
			for x in 0..ne {
				mo[p * ne + x] += w * dv[i * ne + x];
			}
		}
	}
	ar.mlp0.load(&mo)?;
	return Ok(());
}

/// The gated shared-expert branch (qwen3.5/next, kimi-linear): an unconditional
/// SiLU-SwiGLU over the normed input `cms`, written to `out`. qwen adds a
/// per-token sigmoid gate (`shexp.gate_inp`) scaling each row; kimi ships no gate
/// (added straight into the MoE output).
fn shared_expert(m: &Model, l: usize, cms: &GpuBuffer, out: &GpuBuffer, t: usize, ar: &Arena) -> Result<()> {
	let (ne, nffs) = (m.hp.ne, m.hp.nffe);
	gpu_gemm_bt_f64(cms, &m.stream(&layer_name(l, "shexp.gate.weight"))?, t, nffs, ne, &ar.g)?;
	gpu_gemm_bt_f64(cms, &m.stream(&layer_name(l, "shexp.up.weight"))?, t, nffs, ne, &ar.u)?;
	gpu_silu_into(&ar.g, t * nffs, &ar.g)?;
	gpu_mul_inplace(&ar.g, t * nffs, &ar.u)?;
	gpu_gemm_bt_f64(&ar.u, &m.stream(&layer_name(l, "shexp.down.weight"))?, t, ne, nffs, out)?;
	if m.big.contains_key(&layer_name(l, "shexp.gate_inp.weight")) {
		gpu_gemm_bt_f64(cms, &m.stream(&layer_name(l, "shexp.gate_inp.weight"))?, t, 1, ne, &ar.d_bt)?;
		gpu_sigmoid_into(&ar.d_bt, t, &ar.d_bt)?;
		gpu_row_scale(out, &ar.d_bt, t, ne, out)?;
	}
	return Ok(());
}

/// The delta-net FFN sub-block: `pre_ff`-normed input, then a dense SwiGLU (qwen3.5)
/// or a MoE (+ shared expert) FFN (qwen3.5-moe/next softmax + gated shexp,
/// kimi-linear sigmoid + `exp_probs_b` bias + plain shexp), residual `resid` into
/// `h_out`. MoE vs dense is per-layer by expert-tensor presence.
fn delta_ffn(
	m: &Model,
	l: usize,
	sp: &Spec,
	resid: &GpuBuffer,
	h_out: &GpuBuffer,
	t: usize,
	ar: &Arena,
) -> Result<()> {
	let ne = m.hp.ne;
	blk_norm(m, sp, l, Nk::PreFf, t, ne, resid, &ar.cms)?;
	if m.layer_is_moe(l) {
		let sigmoid = m.exp_probs_b[l].is_some();
		moe_core(m, l, &ar.cms, t, ar, sigmoid, m.exp_probs_b[l].as_ref())?;
		if m.big.contains_key(&layer_name(l, "shexp.down.weight")) {
			shared_expert(m, l, &ar.cms, &ar.mo, t, ar)?;
			gpu_add_into(&ar.mlp0, &ar.mo, t * ne, &ar.mlp0)?;
		}
	} else {
		ffn(m, l, sp, t, ar, &ar.cms)?;
	}
	gpu_add_into(&ar.mlp0, resid, t * ne, h_out)?;
	return Ok(());
}

/// True if layer `l` carries an attention mixer (its `q_proj` is present, fused
/// or separate). A nemotron FFN-only layer has neither this nor `ssm_in`.
pub(super) fn layer_is_attn(m: &Model, l: usize) -> bool {
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
	norm_key: Nk,
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

/// True if layer `l` is an lfm2 short-conv recurrent layer (its `shortconv_in_proj`
/// is present) rather than an attention layer: the honest per-layer interleave
/// signal, `n_head_kv(l)==0` in `llama.cpp`.
pub(super) fn layer_is_shortconv(m: &Model, l: usize) -> bool {
	return m.big.contains_key(&layer_name(l, "self_attn.shortconv_in_proj.weight"));
}

/// The lfm2 gated short-convolution mixer (lfm2.cpp build_shortconv_block :157-226),
/// writing the `out_proj`-projected output to `out` without the block residual.
/// operator_norm, `in_proj` split into contiguous row-blocks `B|C|x`, the `B*x`
/// pre-conv gate, a causal depthwise conv (`l_cache` taps, no activation), the
/// `C*conv` post-conv gate, then `out_proj`. All scratch is arena-resident.
fn shortconv_mix(m: &Model, l: usize, h_in: &GpuBuffer, out: &GpuBuffer, t: usize, ar: &Arena, dec: &DecCtx) -> Result<()> {
	let ne = m.hp.ne;
	let lc = m.hp.shortconv_l_cache;
	let (cin, cout) = conv_io(dec, 0)?;
	gpu_rmsnorm_f64(h_in, norm_of(m, l, Nk::Input)?, &m.eps, t, ne, &ar.x)?;
	let w_in = m.stream(&layer_name(l, "self_attn.shortconv_in_proj.weight"))?;
	gpu_gemm_bt_f64(&ar.x, &w_in.view(0, ne * ne), t, ne, ne, &ar.q)?;
	gpu_gemm_bt_f64(&ar.x, &w_in.view(ne * ne, ne * ne), t, ne, ne, &ar.k)?;
	gpu_gemm_bt_f64(&ar.x, &w_in.view(2 * ne * ne, ne * ne), t, ne, ne, &ar.v)?;
	gpu_mul_inplace(&ar.v, t * ne, &ar.q)?;
	let conv_w = m.stream(&layer_name(l, "self_attn.shortconv_conv.weight"))?;
	gpu_ssm_conv_causal(&ar.q, &conv_w, None, t, ne, lc, &ar.v, cin, cout)?;
	gpu_mul_inplace(&ar.k, t * ne, &ar.v)?;
	gpu_gemm_bt_f64(&ar.v, &m.stream(&layer_name(l, "self_attn.shortconv_out_proj.weight"))?, t, ne, ne, out)?;
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
	dec: &DecCtx,
) -> Result<()> {
	let ne = m.hp.ne;
	let recur_mix = |out: &GpuBuffer| -> Result<()> {
		return match hy.recur {
			Recur::Mamba1 => mamba1_mix(m, l, h_in, out, t, ar, dec),
			Recur::Mamba2 => mamba2_mix(m, l, h_in, out, t, ar, dec),
			Recur::Plamo2 => plamo2_mix(m, l, h_in, out, t, ar, dec),
			Recur::GatedDelta => gated_delta_mix(m, l, h_in, out, t, ar, dec),
			Recur::Kda => kda_mix(m, l, h_in, out, t, ar, dec),
			Recur::ShortConv => shortconv_mix(m, l, h_in, out, t, ar, dec),
		};
	};
	match hy.mode {
		HyMode::Parallel => {
			attn_block(m, l, &hy.sp, h_in, t, ar, attn_scale, dec)?;
			mamba2_mix(m, l, h_in, &ar.o, t, ar, dec)?;
			gpu_add_into(&ar.attn_out, &ar.o, t * ne, &ar.mlp)?;
			hybrid_ffn(m, l, &hy.sp, Nk::PreFf, &ar.mlp, &ar.mlp, h_out, t, ar)?;
		}
		HyMode::MixerFfn => {
			if layer_is_recur(m, l) {
				recur_mix(&ar.o)?;
				gpu_add_into(&ar.o, h_in, t * ne, &ar.attn_out)?;
			} else {
				attn_block(m, l, &hy.sp, h_in, t, ar, attn_scale, dec)?;
			}
			hybrid_ffn(m, l, &hy.sp, Nk::PreFf, &ar.attn_out, &ar.attn_out, h_out, t, ar)?;
		}
		HyMode::Triage => {
			if layer_is_recur(m, l) {
				recur_mix(&ar.o)?;
				gpu_add_into(&ar.o, h_in, t * ne, h_out)?;
			} else if layer_is_attn(m, l) {
				attn_block(m, l, &hy.sp, h_in, t, ar, attn_scale, dec)?;
				gpu_copy_into(&ar.attn_out, t * ne, h_out)?;
			} else {
				hybrid_ffn(m, l, &hy.sp, Nk::Input, h_in, h_in, h_out, t, ar)?;
			}
		}
		HyMode::Sandwich => {
			if layer_is_recur(m, l) {
				recur_mix(&ar.o)?;
			} else {
				plamo2_attn(m, l, h_in, &ar.o, t, ar, attn_scale, dec)?;
			}
			gpu_rmsnorm_f64(&ar.o, norm_of(m, l, Nk::PostAttn)?, &m.eps, t, ne, &ar.o)?;
			gpu_add_into(&ar.o, h_in, t * ne, &ar.attn_out)?;
			blk_norm(m, &hy.sp, l, Nk::PreFf, t, ne, &ar.attn_out, &ar.cms)?;
			ffn(m, l, &hy.sp, t, ar, &ar.cms)?;
			gpu_rmsnorm_f64(&ar.mlp0, norm_of(m, l, Nk::Pfw)?, &m.eps, t, ne, &ar.mlp0)?;
			gpu_add_into(&ar.mlp0, &ar.attn_out, t * ne, h_out)?;
		}
		HyMode::DeltaNet => {
			if layer_is_delta(m, l) {
				match hy.recur {
					Recur::Kda => kda_mix(m, l, h_in, &ar.o, t, ar, dec)?,
					_gda => gated_delta_mix(m, l, h_in, &ar.o, t, ar, dec)?,
				}
			} else if hy.recur == Recur::Kda {
				kimi_mla_attn(m, l, h_in, &ar.o, t, ar, dec)?;
			} else {
				delta_full_attn(m, l, h_in, &ar.o, t, ar, attn_scale, dec)?;
			}
			gpu_add_into(&ar.o, h_in, t * ne, &ar.attn_out)?;
			delta_ffn(m, l, &hy.sp, &ar.attn_out, h_out, t, ar)?;
		}
		HyMode::ShortConv => {
			if layer_is_shortconv(m, l) {
				recur_mix(&ar.o)?;
				gpu_add_into(&ar.o, h_in, t * ne, &ar.attn_out)?;
			} else {
				attn_block(m, l, &hy.sp, h_in, t, ar, attn_scale, dec)?;
			}
			delta_ffn(m, l, &hy.sp, &ar.attn_out, h_out, t, ar)?;
		}
	}
	return Ok(());
}
