mod common;

use super::{Arena, Model};
use anyhow::{Result, bail};
use common::{
	Ffn, Hy, HyMode, NormK, Recur, Spec, apply_norm, layer_hybrid, layer_mamba, layer_mamba2,
	layer_minicpm3, layer_moe, layer_recurrent, layer_spec, layer_talkie,
};
use gpu_core::memory::GpuBuffer;

#[derive(Clone, Copy)]
enum Comp {
	Dense(Spec),
	Moe(Spec),
	Recurrent,
	Mamba,
	Mamba2,
	Hybrid(Hy),
	Talkie(Spec),
	Minicpm3(Spec),
}

const TABLE: &[(&str, Comp)] = &[
	("afmoe", Comp::Moe(Spec::dense(Ffn::SiluGate).qk())),
	("apertus", Comp::Dense(Spec::dense(Ffn::SiluGate).sandwich())),
	("arcee", Comp::Dense(Spec::dense(Ffn::ReluSqrSeq).bias())),
	("arctic", Comp::Moe(Spec::dense(Ffn::SiluGate))),
	("arwkv7", Comp::Recurrent),
	("baichuan", Comp::Dense(Spec::dense(Ffn::SiluGate))),
	("bailingmoe", Comp::Moe(Spec::dense(Ffn::SiluGate))),
	("bailingmoe2", Comp::Moe(Spec::dense(Ffn::SiluGate).qk())),
	("bert", Comp::Dense(Spec::dense(Ffn::GeluSeq).layer().encoder())),
	("bitnet", Comp::Dense(Spec::dense(Ffn::SiluGate))),
	("bloom", Comp::Dense(Spec::dense(Ffn::GeluSeq).layer().bias().o_bias().alibi().ffn_bias())),
	("chameleon", Comp::Dense(Spec::dense(Ffn::SiluGate).qk())),
	("chatglm", Comp::Dense(Spec::dense(Ffn::SiluGate).bias())),
	("codeshell", Comp::Dense(Spec::dense(Ffn::GeluSeq).layer().bias().o_bias().ffn_bias())),
	("cogvlm", Comp::Dense(Spec::dense(Ffn::SiluGate))),
	("cohere2", Comp::Dense(Spec::dense(Ffn::SiluGate).sandwich())),
	("cohere2moe", Comp::Moe(Spec::dense(Ffn::SiluGate))),
	("command-r", Comp::Dense(Spec::dense(Ffn::SiluGate).layer().parallel().logit_scale())),
	("dbrx", Comp::Moe(Spec::dense(Ffn::SiluGate).layer())),
	("deci", Comp::Dense(Spec::dense(Ffn::SiluGate).o_bias().ffn_bias())),
	("deepseek", Comp::Moe(Spec::dense(Ffn::SiluGate))),
	("deepseek2", Comp::Moe(Spec::dense(Ffn::SiluGate).mla())),
	("deepseek2-ocr", Comp::Dense(Spec::dense(Ffn::SiluGate).encoder())),
	("deepseek32", Comp::Moe(Spec::dense(Ffn::SiluGate).mla())),
	("deepseek4", Comp::Moe(Spec::dense(Ffn::SiluGate))),
	("dflash", Comp::Dense(Spec::dense(Ffn::SiluGate).qk())),
	("dots1", Comp::Moe(Spec::dense(Ffn::SiluGate).qk())),
	("dream", Comp::Dense(Spec::dense(Ffn::SiluGate))),
	("eagle3", Comp::Dense(Spec::dense(Ffn::SiluGate))),
	("ernie4_5", Comp::Dense(Spec::dense(Ffn::SiluGate))),
	("ernie4_5-moe", Comp::Moe(Spec::dense(Ffn::SiluGate))),
	("eurobert", Comp::Dense(Spec::dense(Ffn::SiluGate).encoder())),
	("exaone", Comp::Dense(Spec::dense(Ffn::SiluGate).bias())),
	("exaone4", Comp::Dense(Spec::dense(Ffn::SiluGate).qk().sandwich().no_pre_norm())),
	("exaone-moe", Comp::Moe(Spec::dense(Ffn::SiluGate).qk())),
	("falcon", Comp::Dense(Spec::dense(Ffn::GeluSeq).layer().parallel().attn_norm2())),
	("falcon-h1", Comp::Hybrid(Hy {
		recur: Recur::Mamba2,
		sp: Spec::dense(Ffn::SiluGate).ffn_bias(),
		mode: HyMode::Parallel,
	})),
	("gemma", Comp::Dense(Spec::dense(Ffn::GeluGate).emb_sqrt_ne())),
	("gemma2", Comp::Dense(Spec::dense(Ffn::GeluGate).sandwich().emb_sqrt_ne().final_softcap())),
	("gemma3", Comp::Dense(Spec::dense(Ffn::GeluGate).sandwich().qk())),
	("gemma3n", Comp::Dense(Spec::dense(Ffn::GeluGate).sandwich().qk())),
	("gemma4", Comp::Dense(Spec::dense(Ffn::GeluGate).sandwich().qk())),
	("gemma4-assistant", Comp::Dense(Spec::dense(Ffn::GeluGate).sandwich().qk())),
	("gemma-embedding", Comp::Dense(Spec::dense(Ffn::GeluGate).sandwich().qk())),
	("glm4", Comp::Dense(Spec::dense(Ffn::SiluGate).sandwich())),
	("glm4moe", Comp::Moe(Spec::dense(Ffn::SiluGate).qk())),
	("glm-dsa", Comp::Moe(Spec::dense(Ffn::SiluGate).mla())),
	("gpt2", Comp::Dense(Spec::dense(Ffn::GeluSeq).layer().bias().o_bias().learned_pos().ffn_bias())),
	("gptj", Comp::Dense(Spec::dense(Ffn::GeluSeq))),
	("gptneox", Comp::Dense(Spec::dense(Ffn::GeluSeq).layer().bias().o_bias().ffn_bias())),
	("gpt-oss", Comp::Moe(Spec::dense(Ffn::SiluGate))),
	("granite", Comp::Dense(Spec::dense(Ffn::SiluGate).o_bias().ffn_bias())),
	("granitehybrid", Comp::Hybrid(Hy {
		recur: Recur::Mamba2,
		sp: Spec::dense(Ffn::SiluGate).o_bias().ffn_bias(),
		mode: HyMode::MixerFfn,
	})),
	("granitemoe", Comp::Moe(Spec::dense(Ffn::SiluGate).o_bias().ffn_bias())),
	("grok", Comp::Moe(Spec::dense(Ffn::SiluGate).emb_scale_kv())),
	("grovemoe", Comp::Moe(Spec::dense(Ffn::SiluGate).qk())),
	("hunyuan-dense", Comp::Dense(Spec::dense(Ffn::SiluGate))),
	("hunyuan-moe", Comp::Moe(Spec::dense(Ffn::SiluGate).qk())),
	("hunyuan_vl", Comp::Dense(Spec::dense(Ffn::SiluGate).qk())),
	("hy_v3", Comp::Moe(Spec::dense(Ffn::SiluGate).qk())),
	("internlm2", Comp::Dense(Spec::dense(Ffn::SiluGate))),
	("jais", Comp::Dense(Spec::dense(Ffn::GeluSeq).layer().bias().o_bias().alibi())),
	("jais2", Comp::Dense(Spec::dense(Ffn::ReluSqrSeq).layer().bias().o_bias().ffn_bias())),
	("jamba", Comp::Hybrid(Hy {
		recur: Recur::Mamba1,
		sp: Spec::dense(Ffn::SiluGate).no_rope(),
		mode: HyMode::MixerFfn,
	})),
	("jina-bert-v2", Comp::Dense(Spec::dense(Ffn::GeluSeq).layer().encoder())),
	("jina-bert-v3", Comp::Dense(Spec::dense(Ffn::GeluSeq).encoder())),
	("kimi-linear", Comp::Hybrid(Hy {
		recur: Recur::Kda,
		sp: Spec::dense(Ffn::SiluGate),
		mode: HyMode::DeltaNet,
	})),
	("lfm2", Comp::Hybrid(Hy {
		recur: Recur::ShortConv,
		sp: Spec::dense(Ffn::SiluGate).qk(),
		mode: HyMode::ShortConv,
	})),
	("lfm2moe", Comp::Hybrid(Hy {
		recur: Recur::ShortConv,
		sp: Spec::dense(Ffn::SiluGate).qk(),
		mode: HyMode::ShortConv,
	})),
	("llada", Comp::Dense(Spec::dense(Ffn::SiluGate))),
	("llada-moe", Comp::Moe(Spec::dense(Ffn::SiluGate).qk())),
	("llama", Comp::Dense(Spec::dense(Ffn::SiluGate).o_bias())),
	("llama4", Comp::Moe(Spec::dense(Ffn::SiluGate))),
	("llama-embed", Comp::Dense(Spec::dense(Ffn::SiluGate).encoder())),
	("maincoder", Comp::Dense(Spec::dense(Ffn::SiluGate).qk())),
	("mamba", Comp::Mamba),
	("mamba2", Comp::Mamba2),
	("mellum", Comp::Moe(Spec::dense(Ffn::SiluGate).qk())),
	("mimo2", Comp::Moe(Spec::dense(Ffn::SiluGate))),
	("minicpm", Comp::Dense(Spec::dense(Ffn::SiluGate).emb_scale_kv().residual_scale().o_bias().ffn_bias())),
	("minicpm3", Comp::Minicpm3(Spec::dense(Ffn::SiluGate).emb_scale_const(12.0))),
	("minimax-m2", Comp::Moe(Spec::dense(Ffn::SiluGate).qk())),
	("mistral3", Comp::Dense(Spec::dense(Ffn::SiluGate).o_bias())),
	("mistral4", Comp::Moe(Spec::dense(Ffn::SiluGate).mla())),
	("modern-bert", Comp::Dense(Spec::dense(Ffn::GeluGate).encoder())),
	("mpt", Comp::Dense(Spec::dense(Ffn::GeluSeq).layer().bias().o_bias().alibi())),
	("nemotron", Comp::Dense(Spec::dense(Ffn::ReluSqrSeq).layer().bias().o_bias().ffn_bias())),
	("nemotron_h", Comp::Hybrid(Hy {
		recur: Recur::Mamba2,
		sp: Spec::dense(Ffn::ReluSqrSeq).o_bias().no_rope(),
		mode: HyMode::Triage,
	})),
	("nemotron_h_moe", Comp::Hybrid(Hy {
		recur: Recur::Mamba2,
		sp: Spec::dense(Ffn::ReluSqrSeq).o_bias().no_rope(),
		mode: HyMode::Triage,
	})),
	("neo-bert", Comp::Dense(Spec::dense(Ffn::SiluGate).encoder())),
	("nomic-bert", Comp::Dense(Spec::dense(Ffn::GeluSeq).encoder())),
	("nomic-bert-moe", Comp::Dense(Spec::dense(Ffn::GeluSeq).encoder())),
	("olmo", Comp::Dense(Spec::dense(Ffn::SiluGate).layer().nonparam())),
	("olmo2", Comp::Dense(Spec::dense(Ffn::SiluGate).sandwich())),
	("olmoe", Comp::Moe(Spec::dense(Ffn::SiluGate).qk())),
	("openelm", Comp::Dense(Spec::dense(Ffn::SiluGate).qk())),
	("orion", Comp::Dense(Spec::dense(Ffn::GeluSeq).layer().bias().ffn_bias())),
	("paddleocr", Comp::Dense(Spec::dense(Ffn::SiluGate).o_bias())),
	("pangu-embedded", Comp::Dense(Spec::dense(Ffn::SiluGate).o_bias().out_bias())),
	("phi2", Comp::Dense(Spec::dense(Ffn::GeluSeq).layer().bias().o_bias().out_bias().parallel().ffn_bias())),
	("phi3", Comp::Moe(Spec::dense(Ffn::SiluGate).out_bias())),
	("phimoe", Comp::Moe(Spec::dense(Ffn::SiluGate))),
	("plamo", Comp::Dense(Spec::dense(Ffn::SiluGate).parallel())),
	("plamo2", Comp::Hybrid(Hy {
		recur: Recur::Plamo2,
		sp: Spec::dense(Ffn::SiluGate),
		mode: HyMode::Sandwich,
	})),
	("plamo3", Comp::Dense(Spec::dense(Ffn::SiluGate).qk().sandwich())),
	("plm", Comp::Dense(Spec::dense(Ffn::ReluSqrSeq).bias())),
	("qwen", Comp::Dense(Spec::dense(Ffn::SiluGate))),
	("qwen2", Comp::Dense(Spec::dense(Ffn::SiluGate).bias().out_bias())),
	("qwen2moe", Comp::Moe(Spec::dense(Ffn::SiluGate))),
	("qwen2vl", Comp::Dense(Spec::dense(Ffn::SiluGate).bias())),
	("qwen3", Comp::Dense(Spec::dense(Ffn::SiluGate).qk())),
	("qwen35", Comp::Hybrid(Hy {
		recur: Recur::GatedDelta,
		sp: Spec::dense(Ffn::SiluGate),
		mode: HyMode::DeltaNet,
	})),
	("qwen35moe", Comp::Hybrid(Hy {
		recur: Recur::GatedDelta,
		sp: Spec::dense(Ffn::SiluGate),
		mode: HyMode::DeltaNet,
	})),
	("qwen3moe", Comp::Moe(Spec::dense(Ffn::SiluGate).qk())),
	("qwen3next", Comp::Hybrid(Hy {
		recur: Recur::GatedDelta,
		sp: Spec::dense(Ffn::SiluGate),
		mode: HyMode::DeltaNet,
	})),
	("qwen3vl", Comp::Dense(Spec::dense(Ffn::SiluGate).qk())),
	("qwen3vlmoe", Comp::Moe(Spec::dense(Ffn::SiluGate).qk())),
	("refact", Comp::Dense(Spec::dense(Ffn::SiluGate))),
	("rnd1", Comp::Moe(Spec::dense(Ffn::SiluGate).qk())),
	("rwkv6", Comp::Recurrent),
	("rwkv6qwen2", Comp::Recurrent),
	("rwkv7", Comp::Recurrent),
	("seed_oss", Comp::Dense(Spec::dense(Ffn::SiluGate))),
	("smallthinker", Comp::Moe(Spec::dense(Ffn::SiluGate))),
	("smollm3", Comp::Dense(Spec::dense(Ffn::SiluGate))),
	("stablelm", Comp::Dense(Spec::dense(Ffn::SiluGate).qk().layer())),
	("starcoder", Comp::Dense(Spec::dense(Ffn::GeluSeq).layer().bias().o_bias().learned_pos().ffn_bias())),
	("starcoder2", Comp::Dense(Spec::dense(Ffn::GeluSeq).layer().bias().o_bias().ffn_bias())),
	("step35", Comp::Moe(Spec::dense(Ffn::SiluGate).qk())),
	("t5", Comp::Dense(Spec::dense(Ffn::GeluSeq).encoder())),
	("t5encoder", Comp::Dense(Spec::dense(Ffn::GeluSeq).encoder())),
	("talkie", Comp::Talkie(Spec::dense(Ffn::SiluGate).logit_scale().embd_skip())),
	("wavtokenizer-dec", Comp::Dense(Spec::dense(Ffn::GeluSeq).encoder())),
	("xverse", Comp::Dense(Spec::dense(Ffn::SiluGate))),
];

const fn supported_names() -> [&'static str; TABLE.len()] {
	let mut names = [""; TABLE.len()];
	let mut i = 0;
	while i < TABLE.len() {
		names[i] = TABLE[i].0;
		i += 1;
	}
	names
}

pub(super) const COMPOSABLE: &[&str] = &supported_names();

pub(super) const VERIFIED: &[&str] = &[
	"arcee", "arctic", "baichuan", "bailingmoe", "bailingmoe2", "chatglm",
	"cogvlm", "command-r", "dbrx", "deci", "deepseek", "deepseek2", "deepseek32", "dots1", "dream",
	"ernie4_5", "ernie4_5-moe", "exaone", "falcon-h1", "gemma", "gemma2", "glm-dsa",
	"glm4moe", "granite", "granitemoe", "granitehybrid", "grok", "grovemoe", "hunyuan-dense", "hunyuan-moe",
	"minicpm",
	"hunyuan_vl", "hy_v3", "internlm2", "jamba", "kimi-linear", "lfm2", "lfm2moe",
	"llada", "llada-moe", "llama4",
	"maincoder", "mamba", "mamba2", "minicpm3", "nemotron_h", "nemotron_h_moe",
	"minimax-m2",
	"mistral4", "olmoe", "openelm", "paddleocr", "pangu-embedded", "phi2",
	"phi3", "plamo", "plamo2", "qwen",
	"qwen2", "qwen2moe", "qwen2vl", "qwen3", "qwen35", "qwen35moe", "qwen3moe",
	"qwen3next", "qwen3vl",
	"qwen3vlmoe", "rnd1", "seed_oss", "smallthinker", "smollm3", "talkie", "xverse",
];

pub(super) fn verified(arch: &str) -> bool {
	VERIFIED.contains(&arch)
}

pub(super) fn supported(arch: &str) -> bool {
	COMPOSABLE.contains(&arch)
}

pub(super) fn is_delta_arch(arch: &str) -> bool {
	for &(name, comp) in TABLE {
		if name == arch {
			return matches!(comp, Comp::Hybrid(hy) if hy.mode == HyMode::DeltaNet);
		}
	}
	return false;
}

pub(super) fn norm_is_layer(arch: &str) -> bool {
	for &(name, comp) in TABLE {
		if name == arch {
			return match comp {
				Comp::Dense(sp) | Comp::Moe(sp) | Comp::Talkie(sp) | Comp::Minicpm3(sp) => {
					sp.norm == NormK::Layer
				}
				Comp::Recurrent | Comp::Mamba | Comp::Mamba2 | Comp::Hybrid(_) => false,
			};
		}
	}
	return false;
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
	for &(name, comp) in TABLE {
		if name == arch {
			return Some(comp);
		}
	}
	return None;
}

pub(super) fn arch_has_recurrence(arch: &str) -> bool {
	return matches!(
		comp_of(arch),
		Some(Comp::Recurrent | Comp::Mamba | Comp::Mamba2 | Comp::Hybrid(_))
	);
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
