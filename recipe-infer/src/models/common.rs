
use super::super::{Arena, Model, Nk, layer_name, softmax};
use super::DecCtx;
use anyhow::{Result, anyhow};
use gpu_core::infer_ops::{
	gpu_flash_gqa, gpu_flash_mla, gpu_gemm_bt, gpu_glu_silu, gpu_rmsnorm_f64,
	gpu_rmsnorm_f64_nogamma,
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

use recipe_ir::arch::{Ffn, Hy, HyMode, NormK, Recur, Spec};

fn norm_of<'m>(m: &'m Model, l: usize, key: Nk) -> Result<&'m GpuBuffer> {
	m.norms[l][key as usize]
		.as_ref()
		.ok_or_else(|| anyhow!("{}: layer {l} has no {:?} norm weight", m.hp.arch, key.name()))
}

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

fn rec_of<'a>(dec: &DecCtx<'a>) -> Result<&'a GpuBuffer> {
	return dec.state.rec();
}

fn conv_io<'a>(dec: &DecCtx<'a>, i: usize) -> Result<(&'a GpuBuffer, &'a GpuBuffer)> {
	return dec.state.conv_io(i);
}

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
	gpu_gemm_bt(
		attn_src,
		&m.stream(&layer_name(l, "self_attn.q_proj.weight"))?,
		t,
		qd,
		ne,
		&ar.q,
	)?;
	let wk = m.stream(&layer_name(l, "self_attn.k_proj.weight"))?;
	gpu_gemm_bt(attn_src, &wk, t, kd, ne, &ar.k)?;
	gpu_gemm_bt(
		attn_src,
		&m.stream(&layer_name(l, "self_attn.v_proj.weight"))?,
		t,
		kd,
		ne,
		&ar.v,
	)?;
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
	gpu_gemm_bt(
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
	gpu_gemm_bt(&ar.x, &m.stream(&layer_name(l, "self_attn.q_a_proj.weight"))?, t, qlr, ne, &ar.mqa)?;
	gpu_rmsnorm_f64(&ar.mqa, norm_of(m, l, Nk::QANorm)?, &m.eps, t, qlr, &ar.mqa)?;
	gpu_gemm_bt(&ar.mqa, &m.stream(&layer_name(l, "self_attn.q_b_proj.weight"))?, t, nqh * hdk, qlr, &ar.mqb)?;
	let qpe_view = ar.mqb.view(nope, t * nqh * hdk - nope);
	gpu_rope_partial_pos(theta, t * nqh, hdk, rot, nqh, pos_base, &qpe_view)?;
	gpu_slice_lead_into(&ar.mqb, t * nqh, hdk, nope, &ar.mqn)?;
	gpu_slice_cols(&ar.mqb, t * nqh, hdk, nope, rot, &ar.mqp)?;
	gpu_gemm_bt(&ar.mqn, &m.stream(&layer_name(l, "self_attn.k_b_proj.weight"))?, t, kvlr, nope, &ar.mqx)?;
	gpu_concat_into(&ar.mqx, &ar.mqp, t, kvlr, rot, &ar.mqc)?;
	gpu_gemm_bt(&ar.x, &m.stream(&layer_name(l, "self_attn.kv_a_proj.weight"))?, t, kvlr + rot, ne, &ar.mkv)?;
	gpu_slice_lead_into(&ar.mkv, t, kvlr + rot, kvlr, &ar.mkc)?;
	gpu_slice_cols(&ar.mkv, t, kvlr + rot, kvlr, rot, &ar.mkp)?;
	gpu_rmsnorm_f64(&ar.mkc, norm_of(m, l, Nk::KvANorm)?, &m.eps, t, kvlr, &ar.mkc)?;
	gpu_rope_partial_pos(theta, t, rot, rot, 1, pos_base, &ar.mkp)?;
	gpu_concat_into(&ar.mkc, &ar.mkp, t, kvlr, rot, &ar.mkk)?;
	gpu_scale_f64_inplace(&m.attn_scale_mla, t * (kvlr + rot), &ar.mqc)?;
	let kw = kvlr + rot;
	cached_mla(&dec, ar, &ar.mqc, &ar.mkk, &ar.mkc, t, nqh, kw, kvlr, &ar.mrw)?;
	gpu_gemm_bt(&ar.mrw, &m.stream(&layer_name(l, "self_attn.v_b_proj.weight"))?, t, hdv, kvlr, &ar.mav)?;
	gpu_gemm_bt(&ar.mav, &m.stream(&layer_name(l, "self_attn.o_proj.weight"))?, t, ne, nqh * hdv, &ar.o)?;
	gpu_add_into(&ar.o, h_in, t * ne, &ar.attn_out)?;
	return Ok(());
}


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
	gpu_gemm_bt(&ar.x, &m.stream(&layer_name(l, "self_attn.q_a_proj.weight"))?, t, qlr, ne, &ar.mqa)?;
	gpu_rmsnorm_f64(&ar.mqa, norm_of(m, l, Nk::QANorm)?, &m.eps, t, qlr, &ar.mqa)?;
	gpu_gemm_bt(&ar.mqa, &m.stream(&layer_name(l, "self_attn.q_b_proj.weight"))?, t, nqh * hdk, qlr, &ar.mqb)?;
	rope_maybe_factors_pos(theta, factors, t * nqh, hdk, rot, nqh, pos_base, &ar.mqb)?;
	gpu_gemm_bt(&ar.x, &m.stream(&layer_name(l, "self_attn.kv_a_proj.weight"))?, t, kvlr + rot, ne, &ar.mkv)?;
	gpu_slice_lead_into(&ar.mkv, t, kvlr + rot, kvlr, &ar.mkc)?;
	gpu_slice_cols(&ar.mkv, t, kvlr + rot, kvlr, rot, &ar.mkp)?;
	gpu_rmsnorm_f64(&ar.mkc, norm_of(m, l, Nk::KvANorm)?, &m.eps, t, kvlr, &ar.mkc)?;
	rope_maybe_factors_pos(theta, factors, t, rot, rot, 1, pos_base, &ar.mkp)?;
	gpu_gemm_bt(&ar.mkc, &m.stream(&layer_name(l, "self_attn.kv_b_proj.weight"))?, t, nqh * hdv, kvlr, &ar.v)?;
	gpu_copy_into(&ar.mkp, t * rot, &ar.k)?;
	for h in 1..nqh {
		gpu_concat_into(&ar.k, &ar.mkp, t, h * rot, rot, &ar.mkk)?;
		gpu_copy_into(&ar.mkk, t * (h + 1) * rot, &ar.k)?;
	}
	gpu_scale_f64_inplace(attn_scale, t * nqh * hdk, &ar.mqb)?;
	cached_gqa(&dec, ar, &ar.mqb, &ar.k, &ar.v, t, nqh, nqh, hdk, kwid, vwid, 0.0, false, &ar.attn)?;
	gpu_gemm_bt(&ar.attn, &m.stream(&layer_name(l, "self_attn.o_proj.weight"))?, t, ne, nqh * hdv, &ar.o)?;
	gpu_scale_f64_inplace(&m.res_scale, t * ne, &ar.o)?;
	gpu_add_into(&ar.o, h_in, t * ne, &ar.attn_out)?;
	gpu_rmsnorm_f64(&ar.attn_out, norm_of(m, l, Nk::PreFf)?, &m.eps, t, ne, &ar.cms)?;
	ffn(m, l, sp, t, ar, &ar.cms)?;
	gpu_scale_f64_inplace(&m.res_scale, t * ne, &ar.mlp0)?;
	gpu_add_into(&ar.mlp0, &ar.attn_out, t * ne, h_out)?;
	return Ok(());
}

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
	gpu_gemm_bt(&ar.x, &m.stream(&layer_name(l, "self_attn.q_proj.weight"))?, t, qd, ne, &ar.q)?;
	gpu_gemm_bt(&ar.x, &m.stream(&layer_name(l, "self_attn.k_proj.weight"))?, t, kd, ne, &ar.k)?;
	gpu_gemm_bt(&ar.x, &m.stream(&layer_name(l, "self_attn.v_proj.weight"))?, t, kd, ne, &ar.v)?;
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
	gpu_gemm_bt(&ar.attn, &m.stream(&layer_name(l, "self_attn.o_proj.weight"))?, t, ne, qd, &ar.o)?;
	gpu_add_into(&ar.o, h_in, t * ne, &ar.attn_out)?;
	gpu_rmsnorm_f64_nogamma(&ar.attn_out, &m.eps, t, ne, &ar.cms)?;
	ffn(m, l, sp, t, ar, &ar.cms)?;
	gpu_add_into(&ar.mlp0, &ar.attn_out, t * ne, &ar.o)?;
	gpu_copy_into(&ar.embd_skip, t * ne, &ar.cms)?;
	gpu_scale_f64_inplace(&m.ls_dev[l], t * ne, &ar.cms)?;
	gpu_add_into(&ar.cms, &ar.o, t * ne, h_out)?;
	return Ok(());
}

fn ffn(m: &Model, l: usize, sp: &Spec, t: usize, ar: &Arena, cms: &GpuBuffer) -> Result<()> {
	let hp = &m.hp;
	let (ne, nff) = (hp.ne, hp.dims[l].nff);
	let up = m.stream(&layer_name(l, "mlp.up_proj.weight"))?;
	gpu_gemm_bt(cms, &up, t, nff, ne, &ar.u)?;
	if sp.ffn_bias
		&& let Some(b) = &m.ffn_up_bias[l]
	{
		gpu_bias_add(&ar.u, b, t, nff, &ar.u)?;
	}
	match sp.ffn {
		Ffn::SiluGate => {
			gpu_gemm_bt(
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
			gpu_gemm_bt(
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
	gpu_gemm_bt(
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
	gpu_gemm_bt(
		&ar.x,
		&m.stream(&layer_name(l, "self_attn.q_proj.weight"))?,
		t,
		qd,
		ne,
		&ar.q,
	)?;
	gpu_gemm_bt(
		&ar.x,
		&m.stream(&layer_name(l, "self_attn.k_proj.weight"))?,
		t,
		kd,
		ne,
		&ar.k,
	)?;
	gpu_gemm_bt(
		&ar.x,
		&m.stream(&layer_name(l, "self_attn.v_proj.weight"))?,
		t,
		kd,
		ne,
		&ar.v,
	)?;
	gpu_silu_into(&ar.k, t * kd, &ar.k)?;
	gpu_scan_linear_recurrence(&ar.k, &ar.v, t, kd, &ar.attn, rec_of(dec)?)?;
	gpu_gemm_bt(
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
	gpu_gemm_bt(&ar.x, &w_in.view(0, di * ne), t, di, ne, &ar.ss_x)?;
	gpu_gemm_bt(&ar.x, &w_in.view(di * ne, di * ne), t, di, ne, &ar.ss_z)?;

	let conv_w = ssm_of(&m.ssm_conv_w, l, arch, "ssm_conv1d.weight")?;
	let conv_b = ssm_of(&m.ssm_conv_b, l, arch, "ssm_conv1d.bias")?;
	gpu_ssm_conv_causal_silu(&ar.ss_x, conv_w, Some(conv_b), t, di, dc, &ar.ss_xc, cin, cout)?;

	let w_x = m.stream(&layer_name(l, "self_attn.ssm_x.weight"))?;
	gpu_gemm_bt(&ar.ss_xc, &w_x, t, dbw, di, &ar.ss_db)?;
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
	gpu_gemm_bt(&ar.ss_dtlr, &w_dt, t, di, dr, &ar.ss_dt)?;
	let dt_b = ssm_of(&m.ssm_dt_b, l, arch, "ssm_dt.bias")?;
	gpu_bias_add(&ar.ss_dt, dt_b, t, di, &ar.ss_dt)?;

	let a = ssm_of(&m.ssm_a, l, arch, "ssm_a")?;
	let d = ssm_of(&m.ssm_d, l, arch, "ssm_d")?;
	gpu_ssm_scan_mamba1(&ar.ss_xc, &ar.ss_dt, a, &ar.ss_bb, &ar.ss_cc, d, t, di, ds, &ar.ss_y, rec_of(dec)?)?;

	gpu_silu_into(&ar.ss_z, t * di, &ar.ss_z)?;
	gpu_mul_inplace(&ar.ss_z, t * di, &ar.ss_y)?;

	let w_out = m.stream(&layer_name(l, "self_attn.ssm_out.weight"))?;
	gpu_gemm_bt(&ar.ss_y, &w_out, t, ne, di, out)?;
	return Ok(());
}

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
	gpu_gemm_bt(&ar.x, &w_in.view(0, di * ne), t, di, ne, &ar.ss_z)?;
	gpu_gemm_bt(&ar.x, &w_in.view(di * ne, conv_dim * ne), t, conv_dim, ne, &ar.ss_xbc)?;
	gpu_gemm_bt(&ar.x, &w_in.view((di + conv_dim) * ne, nh * ne), t, nh, ne, &ar.ss_dtlr)?;

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
	gpu_gemm_bt(&ar.ss_y, &w_out, t, ne, di, out)?;
	return Ok(());
}

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
	gpu_gemm_bt(&ar.x, &w_in, t, 2 * di, ne, &ar.ss_zx)?;
	deinterleave_heads(&ar.ss_zx, t, head_dim, nh, head_dim, &ar.ss_x, &ar.ss_dt, &ar.ss_y)?;
	deinterleave_heads(&ar.ss_zx, t, head_dim, nh, 0, &ar.ss_z, &ar.ss_dt, &ar.ss_y)?;

	let conv_w = ssm_of(&m.ssm_conv_w, l, arch, "ssm_conv1d.weight")?;
	let conv_b = ssm_of(&m.ssm_conv_b, l, arch, "ssm_conv1d.bias")?;
	gpu_ssm_conv_causal_silu(&ar.ss_x, conv_w, Some(conv_b), t, di, dc, &ar.ss_xc, cin, cout)?;

	let w_x = m.stream(&layer_name(l, "self_attn.ssm_x.weight"))?;
	gpu_gemm_bt(&ar.ss_xc, &w_x, t, bcd, di, &ar.ss_db)?;
	gpu_slice_cols(&ar.ss_db, t, bcd, 0, ds, &ar.ss_bb)?;
	gpu_slice_cols(&ar.ss_db, t, bcd, ds, ds, &ar.ss_cc)?;
	gpu_slice_cols(&ar.ss_db, t, bcd, 2 * ds, dtd, &ar.ss_dtlr)?;

	gpu_rmsnorm_f64(&ar.ss_bb, ssm_of(&m.ssm_b_norm, l, arch, "ssm_b_norm")?, &m.eps, t, ds, &ar.ss_bb)?;
	gpu_rmsnorm_f64(&ar.ss_cc, ssm_of(&m.ssm_c_norm, l, arch, "ssm_c_norm")?, &m.eps, t, ds, &ar.ss_cc)?;
	gpu_rmsnorm_f64(&ar.ss_dtlr, ssm_of(&m.ssm_dt_norm, l, arch, "ssm_dt_norm")?, &m.eps, t, dtd, &ar.ss_dtlr)?;

	let dt = ar.ss_dt.view(0, t * nh);
	let w_dt = m.stream(&layer_name(l, "self_attn.ssm_dt.weight"))?;
	gpu_gemm_bt(&ar.ss_dtlr, &w_dt, t, nh, dtd, &dt)?;
	gpu_bias_add(&dt, ssm_of(&m.ssm_dt_b, l, arch, "ssm_dt.bias")?, t, nh, &dt)?;

	gpu_concat_into(&ar.ss_bb, &ar.ss_cc, t, ds, ds, &ar.ss_db)?;
	gpu_concat_into(&ar.ss_xc, &ar.ss_db.view(0, t * 2 * ds), t, di, 2 * ds, &ar.ss_xbc)?;

	let a = ssm_of(&m.ssm_a, l, arch, "ssm_a")?;
	let d = ssm_of(&m.ssm_d, l, arch, "ssm_d")?;
	gpu_ssm_scan_mamba2(&ar.ss_xbc, &dt, a, d, t, di, ds, nh, 1, conv_dim, &ar.ss_y, rec_of(dec)?)?;

	gpu_silu_into(&ar.ss_z, t * di, &ar.ss_z)?;
	gpu_mul_inplace(&ar.ss_z, t * di, &ar.ss_y)?;

	let w_out = m.stream(&layer_name(l, "self_attn.ssm_out.weight"))?;
	gpu_gemm_bt(&ar.ss_y, &w_out, t, ne, di, out)?;
	return Ok(());
}

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
	gpu_gemm_bt(&ar.x, &m.stream(&layer_name(l, "self_attn.q_proj.weight"))?, t, qd, ne, &ar.q)?;
	gpu_gemm_bt(&ar.x, &m.stream(&layer_name(l, "self_attn.k_proj.weight"))?, t, kd, ne, &ar.k)?;
	gpu_gemm_bt(&ar.x, &m.stream(&layer_name(l, "self_attn.v_proj.weight"))?, t, kd, ne, &ar.v)?;
	gpu_rmsnorm_f64_nogamma(&ar.q, &m.eps, t * nqh, hd, &ar.q)?;
	gpu_broadcast_mul(&ar.q, norm_of(m, l, Nk::QNorm)?, t * qd, qd, &ar.q)?;
	gpu_rmsnorm_f64_nogamma(&ar.k, &m.eps, t * nkv, hd, &ar.k)?;
	gpu_broadcast_mul(&ar.k, norm_of(m, l, Nk::KNorm)?, t * kd, kd, &ar.k)?;
	gpu_rope_partial_pos(&m.theta_full, t * nqh, hd, hd, nqh, pos_base, &ar.q)?;
	gpu_rope_partial_pos(&m.theta_full, t * nkv, hd, hd, nkv, pos_base, &ar.k)?;
	gpu_scale_f64_inplace(attn_scale, t * qd, &ar.q)?;
	cached_gqa(&dec, ar, &ar.q, &ar.k, &ar.v, t, nqh, nkv, hd, kd, kd, 0.0, false, &ar.attn)?;
	gpu_gemm_bt(&ar.attn, &m.stream(&layer_name(l, "self_attn.o_proj.weight"))?, t, ne, qd, out)?;
	return Ok(());
}

pub(super) fn layer_is_recur(m: &Model, l: usize) -> bool {
	return m.big.contains_key(&layer_name(l, "self_attn.ssm_in.weight"));
}

pub(super) fn layer_is_delta(m: &Model, l: usize) -> bool {
	return m.big.contains_key(&layer_name(l, "self_attn.ssm_conv1d.weight"))
		|| m.big.contains_key(&layer_name(l, "self_attn.q_conv.weight"));
}

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

fn gated_delta_mix(m: &Model, l: usize, h_in: &GpuBuffer, out: &GpuBuffer, t: usize, ar: &Arena, dec: &DecCtx) -> Result<()> {
	let hp = &m.hp;
	let arch = hp.arch.as_str();
	let ne = hp.ne;
	let (d, hk, hv, key_dim, value_dim, conv_dim) = delta_dims(m);
	let dc = hp.ssm_d_conv;
	let (cin, cout) = conv_io(dec, 0)?;
	gpu_rmsnorm_f64(h_in, norm_of(m, l, Nk::Input)?, &m.eps, t, ne, &ar.x)?;
	gpu_gemm_bt(&ar.x, &m.stream(&layer_name(l, "self_attn.qkv_proj.weight"))?, t, conv_dim, ne, &ar.d_qkv)?;
	gpu_gemm_bt(&ar.x, &m.stream(&layer_name(l, "self_attn.z_gate.weight"))?, t, value_dim, ne, &ar.d_z)?;
	if m.big.contains_key(&layer_name(l, "self_attn.ssm_ba.weight")) {
		gpu_gemm_bt(&ar.x, &m.stream(&layer_name(l, "self_attn.ssm_ba.weight"))?, t, 2 * hv, ne, &ar.d_o)?;
		deinterleave_heads(&ar.d_o, t, 1, hv, 0, &ar.d_bt, &ar.d_q, &ar.d_k)?;
		deinterleave_heads(&ar.d_o, t, 1, hv, 1, &ar.d_g, &ar.d_q, &ar.d_k)?;
	} else {
		gpu_gemm_bt(&ar.x, &m.stream(&layer_name(l, "self_attn.ssm_beta.weight"))?, t, hv, ne, &ar.d_bt)?;
		gpu_gemm_bt(&ar.x, &m.stream(&layer_name(l, "self_attn.ssm_alpha.weight"))?, t, hv, ne, &ar.d_g)?;
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
	gpu_gemm_bt(&ar.d_o, &m.stream(&layer_name(l, "self_attn.ssm_out.weight"))?, t, ne, value_dim, out)?;
	return Ok(());
}

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
	gpu_gemm_bt(&ar.x, &m.stream(&layer_name(l, "self_attn.q_proj.weight"))?, t, di, ne, &ar.d_qkv)?;
	gpu_ssm_conv_causal_silu(&ar.d_qkv, qcw, None, t, di, dc, &ar.d_q, qin, qout)?;
	let kcw = ssm_of(&m.ssm_k_conv_w, l, arch, "k_conv.weight")?;
	gpu_gemm_bt(&ar.x, &m.stream(&layer_name(l, "self_attn.k_proj.weight"))?, t, di, ne, &ar.d_qkv)?;
	gpu_ssm_conv_causal_silu(&ar.d_qkv, kcw, None, t, di, dc, &ar.d_k, kin, kout)?;
	let vcw = ssm_of(&m.ssm_v_conv_w, l, arch, "v_conv.weight")?;
	gpu_gemm_bt(&ar.x, &m.stream(&layer_name(l, "self_attn.v_proj.weight"))?, t, di, ne, &ar.d_qkv)?;
	gpu_ssm_conv_causal_silu(&ar.d_qkv, vcw, None, t, di, dc, &ar.d_v, vin, vout)?;
	gpu_l2norm_rows(&ar.d_q, &m.eps, t * h, d, &ar.d_q)?;
	gpu_l2norm_rows(&ar.d_k, &m.eps, t * h, d, &ar.d_k)?;
	gpu_gemm_bt(&ar.x, &m.stream(&layer_name(l, "self_attn.f_a.weight"))?, t, d, ne, &ar.d_z)?;
	gpu_gemm_bt(&ar.d_z, &m.stream(&layer_name(l, "self_attn.f_b.weight"))?, t, di, d, &ar.d_g)?;
	gpu_bias_add(&ar.d_g, ssm_of(&m.ssm_dt_b, l, arch, "ssm_dt.bias")?, t, di, &ar.d_g)?;
	gpu_softplus(&ar.d_g, t * di, &ar.d_g)?;
	gpu_gemm_bt(&ar.x, &m.stream(&layer_name(l, "self_attn.ssm_beta.weight"))?, t, h, ne, &ar.d_bt)?;
	gpu_sigmoid_into(&ar.d_bt, t * h, &ar.d_bt)?;
	let a = ssm_of(&m.ssm_a, l, arch, "ssm_a")?;
	let scale = 1.0 / (d as f64).sqrt();
	gpu_gated_delta_scan(&ar.d_q, &ar.d_k, &ar.d_v, &ar.d_g, &ar.d_bt, a, &ar.d_o, t, h, d, true, scale, rec_of(dec)?)?;
	gpu_gemm_bt(&ar.x, &m.stream(&layer_name(l, "self_attn.g_a.weight"))?, t, d, ne, &ar.d_z)?;
	gpu_gemm_bt(&ar.d_z, &m.stream(&layer_name(l, "self_attn.g_b.weight"))?, t, di, d, &ar.d_qkv)?;
	gpu_sigmoid_into(&ar.d_qkv, t * di, &ar.d_qkv)?;
	gpu_rmsnorm_f64(&ar.d_o, ssm_of(&m.ssm_norm, l, arch, "ssm_norm.weight")?, &m.eps, t * h, d, &ar.d_o)?;
	gpu_mul_inplace(&ar.d_qkv, t * di, &ar.d_o)?;
	gpu_gemm_bt(&ar.d_o, &m.stream(&layer_name(l, "self_attn.o_proj.weight"))?, t, ne, di, out)?;
	return Ok(());
}

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
	gpu_gemm_bt(&ar.x, &m.stream(&layer_name(l, "self_attn.q_proj.weight"))?, t, 2 * qd, ne, &ar.d_qkv)?;
	deinterleave_heads(&ar.d_qkv, t, hd, nqh, 0, &ar.q, &ar.d_q, &ar.d_k)?;
	deinterleave_heads(&ar.d_qkv, t, hd, nqh, hd, &ar.d_z, &ar.d_q, &ar.d_k)?;
	gpu_gemm_bt(&ar.x, &m.stream(&layer_name(l, "self_attn.k_proj.weight"))?, t, kd, ne, &ar.k)?;
	gpu_gemm_bt(&ar.x, &m.stream(&layer_name(l, "self_attn.v_proj.weight"))?, t, kd, ne, &ar.v)?;
	gpu_rmsnorm_f64(&ar.q, norm_of(m, l, Nk::QNorm)?, &m.eps, t * nqh, hd, &ar.q)?;
	gpu_rmsnorm_f64(&ar.k, norm_of(m, l, Nk::KNorm)?, &m.eps, t * nkv, hd, &ar.k)?;
	gpu_rope_partial_pos(&m.theta_full, t * nqh, hd, hd, nqh, pos_base, &ar.q)?;
	gpu_rope_partial_pos(&m.theta_full, t * nkv, hd, hd, nkv, pos_base, &ar.k)?;
	gpu_scale_f64_inplace(attn_scale, t * qd, &ar.q)?;
	cached_gqa(&dec, ar, &ar.q, &ar.k, &ar.v, t, nqh, nkv, hd, kd, kd, 0.0, false, &ar.attn)?;
	gpu_sigmoid_into(&ar.d_z, t * qd, &ar.d_z)?;
	gpu_mul_inplace(&ar.d_z, t * qd, &ar.attn)?;
	gpu_gemm_bt(&ar.attn, &m.stream(&layer_name(l, "self_attn.o_proj.weight"))?, t, ne, qd, out)?;
	return Ok(());
}

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
	gpu_gemm_bt(&ar.x, &m.stream(&layer_name(l, "self_attn.q_proj.weight"))?, t, nqh * hdk, ne, &ar.mqb)?;
	gpu_slice_lead_into(&ar.mqb, t * nqh, hdk, nope, &ar.mqn)?;
	gpu_slice_cols(&ar.mqb, t * nqh, hdk, nope, rot, &ar.mqp)?;
	gpu_gemm_bt(&ar.mqn, &m.stream(&layer_name(l, "self_attn.k_b_proj.weight"))?, t, kvlr, nope, &ar.mqx)?;
	gpu_concat_into(&ar.mqx, &ar.mqp, t, kvlr, rot, &ar.mqc)?;
	gpu_gemm_bt(&ar.x, &m.stream(&layer_name(l, "self_attn.kv_a_proj.weight"))?, t, kvlr + rot, ne, &ar.mkv)?;
	gpu_slice_lead_into(&ar.mkv, t, kvlr + rot, kvlr, &ar.mkc)?;
	gpu_slice_cols(&ar.mkv, t, kvlr + rot, kvlr, rot, &ar.mkp)?;
	gpu_rmsnorm_f64(&ar.mkc, norm_of(m, l, Nk::KvANorm)?, &m.eps, t, kvlr, &ar.mkc)?;
	gpu_concat_into(&ar.mkc, &ar.mkp, t, kvlr, rot, &ar.mkk)?;
	gpu_scale_f64_inplace(&m.attn_scale_mla, t * (kvlr + rot), &ar.mqc)?;
	let kw = kvlr + rot;
	cached_mla(&dec, ar, &ar.mqc, &ar.mkk, &ar.mkc, t, nqh, kw, kvlr, &ar.mrw)?;
	gpu_gemm_bt(&ar.mrw, &m.stream(&layer_name(l, "self_attn.v_b_proj.weight"))?, t, hdv, kvlr, &ar.mav)?;
	gpu_gemm_bt(&ar.mav, &m.stream(&layer_name(l, "self_attn.o_proj.weight"))?, t, ne, nqh * hdv, out)?;
	return Ok(());
}

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
	gpu_gemm_bt(cms, &gate_w, t, nexp, ne, &logits)?;
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
		let gu_w = m.widen_from(&es, 0, 2 * nffe * ne, hp.moe_gu_dt)?;
		gpu_gemm_bt(&ar.moe_xg, &gu_w, np, 2 * nffe, ne, &ar.moe_gu)?;
		gpu_glu_silu(&ar.moe_gu, np, nffe, &ar.moe_ea)?;
		let dn_w = m.widen_from(&es, hp.gu_bytes, ne * nffe, hp.moe_dn_dt)?;
		gpu_gemm_bt(&ar.moe_ea, &dn_w, np, ne, nffe, &ar.moe_dv)?;
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

fn shared_expert(m: &Model, l: usize, cms: &GpuBuffer, out: &GpuBuffer, t: usize, ar: &Arena) -> Result<()> {
	let (ne, nffs) = (m.hp.ne, m.hp.nffe);
	gpu_gemm_bt(cms, &m.stream(&layer_name(l, "shexp.gate.weight"))?, t, nffs, ne, &ar.g)?;
	gpu_gemm_bt(cms, &m.stream(&layer_name(l, "shexp.up.weight"))?, t, nffs, ne, &ar.u)?;
	gpu_silu_into(&ar.g, t * nffs, &ar.g)?;
	gpu_mul_inplace(&ar.g, t * nffs, &ar.u)?;
	gpu_gemm_bt(&ar.u, &m.stream(&layer_name(l, "shexp.down.weight"))?, t, ne, nffs, out)?;
	if m.big.contains_key(&layer_name(l, "shexp.gate_inp.weight")) {
		gpu_gemm_bt(cms, &m.stream(&layer_name(l, "shexp.gate_inp.weight"))?, t, 1, ne, &ar.d_bt)?;
		gpu_sigmoid_into(&ar.d_bt, t, &ar.d_bt)?;
		gpu_row_scale(out, &ar.d_bt, t, ne, out)?;
	}
	return Ok(());
}

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

pub(super) fn layer_is_attn(m: &Model, l: usize) -> bool {
	return m.big.contains_key(&layer_name(l, "self_attn.q_proj.weight"));
}

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

pub(super) fn layer_is_shortconv(m: &Model, l: usize) -> bool {
	return m.big.contains_key(&layer_name(l, "self_attn.shortconv_in_proj.weight"));
}

fn shortconv_mix(m: &Model, l: usize, h_in: &GpuBuffer, out: &GpuBuffer, t: usize, ar: &Arena, dec: &DecCtx) -> Result<()> {
	let ne = m.hp.ne;
	let lc = m.hp.shortconv_l_cache;
	let (cin, cout) = conv_io(dec, 0)?;
	gpu_rmsnorm_f64(h_in, norm_of(m, l, Nk::Input)?, &m.eps, t, ne, &ar.x)?;
	let w_in = m.stream(&layer_name(l, "self_attn.shortconv_in_proj.weight"))?;
	gpu_gemm_bt(&ar.x, &w_in.view(0, ne * ne), t, ne, ne, &ar.q)?;
	gpu_gemm_bt(&ar.x, &w_in.view(ne * ne, ne * ne), t, ne, ne, &ar.k)?;
	gpu_gemm_bt(&ar.x, &w_in.view(2 * ne * ne, ne * ne), t, ne, ne, &ar.v)?;
	gpu_mul_inplace(&ar.v, t * ne, &ar.q)?;
	let conv_w = m.stream(&layer_name(l, "self_attn.shortconv_conv.weight"))?;
	gpu_ssm_conv_causal(&ar.q, &conv_w, None, t, ne, lc, &ar.v, cin, cout)?;
	gpu_mul_inplace(&ar.k, t * ne, &ar.v)?;
	gpu_gemm_bt(&ar.v, &m.stream(&layer_name(l, "self_attn.shortconv_out_proj.weight"))?, t, ne, ne, out)?;
	return Ok(());
}

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
