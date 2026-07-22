mod common;

use super::{Arena, Model};
use anyhow::{Result, bail};
use common::{
	apply_norm, layer_hybrid, layer_mamba, layer_mamba2, layer_minicpm3, layer_moe,
	layer_recurrent, layer_spec, layer_talkie,
};
use gpu_core::memory::GpuBuffer;
use recipe_ir::arch::{Comp, Hy, HyMode, NormK, Recur, Spec, TABLE};

pub(super) const fn supported_names() -> [&'static str; TABLE.len()] {
	return recipe_ir::arch::supported_names();
}

pub(super) const COMPOSABLE: &[&str] = &supported_names();

pub(super) const VERIFIED: &[&str] = recipe_ir::arch::VERIFIED;

pub(super) fn verified(arch: &str) -> bool {
	return recipe_ir::arch::verified(arch);
}

pub(super) fn supported(arch: &str) -> bool {
	return recipe_ir::arch::supported(arch);
}

pub(super) fn is_delta_arch(arch: &str) -> bool {
	return recipe_ir::arch::is_delta_arch(arch);
}

pub(super) fn norm_is_layer(arch: &str) -> bool {
	return recipe_ir::arch::norm_is_layer(arch);
}

#[derive(Clone, Copy)]
pub(super) struct DecCtx<'a> {
	pub(super) cached: usize,
	pub(super) win_base: usize,
	pub(super) win: usize,
	pub(super) state: &'a crate::llm::LayerCache,
	pub(super) stage: &'a crate::llm::KvStage,
}

fn comp_of(arch: &str) -> Option<Comp> {
	return recipe_ir::arch::comp_of(arch);
}

pub(super) fn arch_has_recurrence(arch: &str) -> bool {
	return recipe_ir::arch::arch_has_recurrence(arch);
}

pub(super) struct LayerCacheShape {
	pub(super) kw: usize,
	pub(super) vw: usize,
	pub(super) rec: usize,
	pub(super) conv: Vec<usize>,
}

fn attn_shape(m: &Model, l: usize) -> LayerCacheShape {
	let d = &m.hp.dims[l];
	let w = d.nkv * d.hd;
	return LayerCacheShape { kw: w, vw: w, rec: 0, conv: Vec::new() };
}

fn mla_shape(m: &Model) -> LayerCacheShape {
	return LayerCacheShape {
		kw: m.hp.kv_lora_rank + m.hp.n_rot,
		vw: m.hp.kv_lora_rank,
		rec: 0,
		conv: Vec::new(),
	};
}

pub(super) fn layer_cache_shape(m: &Model, l: usize) -> LayerCacheShape {
	let hp = &m.hp;
	let empty = LayerCacheShape { kw: 0, vw: 0, rec: 0, conv: Vec::new() };
	let dc = hp.ssm_d_conv;
	match comp_of(hp.arch.as_str()) {
		None => empty,
		Some(Comp::Dense(sp)) | Some(Comp::Moe(sp)) => {
			if sp.mla { mla_shape(m) } else { attn_shape(m, l) }
		}
		Some(Comp::Talkie(_)) => attn_shape(m, l),
		Some(Comp::Minicpm3(_)) => {
			let nqh = hp.dims[l].nqh;
			LayerCacheShape {
				kw: nqh * hp.head_k_mla,
				vw: nqh * hp.head_v_mla,
				rec: 0,
				conv: Vec::new(),
			}
		}
		Some(Comp::Recurrent) => {
			let d = &hp.dims[l];
			LayerCacheShape { kw: 0, vw: 0, rec: d.nkv * d.hd, conv: Vec::new() }
		}
		Some(Comp::Mamba) => {
			let (di, ds) = (hp.ssm_d_inner, hp.ssm_d_state);
			LayerCacheShape { kw: 0, vw: 0, rec: di * ds, conv: vec![(dc - 1) * di] }
		}
		Some(Comp::Mamba2) => {
			let (di, ds, ng) = (hp.ssm_d_inner, hp.ssm_d_state, hp.ssm_n_group.max(1));
			let cd = di + 2 * ng * ds;
			LayerCacheShape { kw: 0, vw: 0, rec: di * ds, conv: vec![(dc - 1) * cd] }
		}
		Some(Comp::Hybrid(hy)) => hybrid_layer_shape(m, l, &hy),
	}
}

fn hybrid_layer_shape(m: &Model, l: usize, hy: &Hy) -> LayerCacheShape {
	let hp = &m.hp;
	let dc = hp.ssm_d_conv;
	if hy.mode == HyMode::Parallel {
		let d = &hp.dims[l];
		let w = d.nkv * d.hd;
		let (di, ds, ng) = (hp.ssm_d_inner, hp.ssm_d_state, hp.ssm_n_group.max(1));
		let cd = di + 2 * ng * ds;
		return LayerCacheShape { kw: w, vw: w, rec: di * ds, conv: vec![(dc - 1) * cd] };
	}
	if common::layer_is_shortconv(m, l) {
		let lc = hp.shortconv_l_cache;
		return LayerCacheShape { kw: 0, vw: 0, rec: 0, conv: vec![(lc - 1) * hp.ne] };
	}
	if common::layer_is_delta(m, l) {
		let (d, _hk, hv, _key, _val, conv_dim) = common::delta_dims(m);
		return match hy.recur {
			Recur::Kda => {
				let di = d * hv;
				LayerCacheShape {
					kw: 0,
					vw: 0,
					rec: hv * d * d,
					conv: vec![(dc - 1) * di, (dc - 1) * di, (dc - 1) * di],
				}
			}
			_gda => LayerCacheShape { kw: 0, vw: 0, rec: hv * d * d, conv: vec![(dc - 1) * conv_dim] },
		};
	}
	if common::layer_is_recur(m, l) {
		let (di, ds, ng) = (hp.ssm_d_inner, hp.ssm_d_state, hp.ssm_n_group.max(1));
		return match hy.recur {
			Recur::Mamba2 => {
				let cd = di + 2 * ng * ds;
				LayerCacheShape { kw: 0, vw: 0, rec: di * ds, conv: vec![(dc - 1) * cd] }
			}
			_mamba1_or_plamo2 => {
				LayerCacheShape { kw: 0, vw: 0, rec: di * ds, conv: vec![(dc - 1) * di] }
			}
		};
	}
	if common::layer_is_attn(m, l) {
		if hy.recur == Recur::Kda {
			return mla_shape(m);
		}
		return attn_shape(m, l);
	}
	return LayerCacheShape { kw: 0, vw: 0, rec: 0, conv: Vec::new() };
}


fn spec_of(m: &Model) -> Option<Spec> {
	let arch = m.hp.arch.as_str();
	for &(name, comp) in TABLE {
		if name == arch {
			return match comp {
				Comp::Dense(sp) | Comp::Moe(sp) | Comp::Talkie(sp) | Comp::Minicpm3(sp) => Some(sp),
				Comp::Recurrent | Comp::Mamba | Comp::Mamba2 | Comp::Hybrid(_) => None,
			};
		}
	}
	return None;
}

pub(super) fn embedding_scale(m: &Model) -> f64 {
	match spec_of(m) {
		Some(sp) if sp.emb_sqrt_ne => (m.hp.ne as f64).sqrt(),
		Some(sp) if sp.emb_scale_kv && m.hp.embedding_scale > 0.0 => m.hp.embedding_scale,
		Some(sp) if sp.emb_scale_const != 1.0 => sp.emb_scale_const,
		_other => 1.0,
	}
}

pub(super) fn out_bias(m: &Model) -> bool {
	spec_of(m).is_some_and(|sp| sp.out_bias)
}

pub(super) fn final_softcap(m: &Model) -> f64 {
	match spec_of(m) {
		Some(sp) if sp.final_softcap => m.hp.softcap,
		_other => 0.0,
	}
}

pub(super) fn embd_skip(m: &Model) -> bool {
	return spec_of(m).is_some_and(|sp| sp.embd_skip);
}

pub(super) fn logit_scale(m: &Model) -> f64 {
	match spec_of(m) {
		Some(sp) if sp.logit_scale && m.hp.logit_scale != 0.0 => m.hp.logit_scale,
		_other => 1.0,
	}
}

pub(super) fn decoder_norm(
	m: &Model,
	x: &GpuBuffer,
	rows: usize,
	ne: usize,
	out: &GpuBuffer,
) -> Result<()> {
	let kind = spec_of(m).map_or(NormK::Rms, |sp| sp.norm);
	let gamma = if m.big.contains_key("model.decoder.norm.weight") {
		Some(&m.decoder_norm)
	} else {
		None
	};
	return apply_norm(kind, gamma, m.decoder_norm_b.as_ref(), &m.eps, rows, ne, x, out);
}

pub(super) fn dispatch(
	m: &Model,
	l: usize,
	h_in: &GpuBuffer,
	h_out: &GpuBuffer,
	t: usize,
	ar: &Arena,
	attn_scale: &GpuBuffer,
	dec: &DecCtx,
) -> Result<()> {
	let arch = m.hp.arch.as_str();
	for &(name, comp) in TABLE {
		if name == arch {
			return match comp {
				Comp::Dense(sp) | Comp::Moe(sp) if m.layer_is_moe(l) => {
					layer_moe(m, l, &sp, h_in, h_out, t, ar, attn_scale, dec)
				}
				Comp::Dense(sp) | Comp::Moe(sp) => {
					layer_spec(m, l, &sp, h_in, h_out, t, ar, attn_scale, dec)
				}
				Comp::Recurrent => layer_recurrent(m, l, h_in, h_out, t, ar, attn_scale, dec),
				Comp::Mamba => layer_mamba(m, l, h_in, h_out, t, ar, dec),
				Comp::Mamba2 => layer_mamba2(m, l, h_in, h_out, t, ar, dec),
				Comp::Hybrid(hy) => layer_hybrid(m, l, &hy, h_in, h_out, t, ar, attn_scale, dec),
				Comp::Talkie(sp) => layer_talkie(m, l, &sp, h_in, h_out, t, ar, attn_scale, dec),
				Comp::Minicpm3(sp) => layer_minicpm3(m, l, &sp, h_in, h_out, t, ar, attn_scale, dec),
			};
		}
	}
	bail!(
		"unsupported architecture {arch:?}: no decode composition in models/; \
		 add its TABLE entry"
	)
}
