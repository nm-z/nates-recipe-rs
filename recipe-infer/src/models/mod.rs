//! Flat declarative arch table: each GGUF `general.architecture` string maps to
//! a [`Comp`] composition entry, and [`dispatch`] is a table lookup that composes
//! the block via the shared `common` drivers. The runtime composes the arch from
//! shared ops and transfers; it does not implement any arch imperatively.
mod common;

use super::{Arena, Model};
use anyhow::{Result, bail};
use common::{
	Ffn, Hy, HyMode, NormK, Recur, Spec, apply_norm, layer_hybrid, layer_mamba, layer_mamba2,
	layer_minicpm3, layer_moe, layer_recurrent, layer_spec, layer_talkie,
};
use gpu_core::memory::GpuBuffer;

/// One architecture's decode composition: a dense [`Spec`] block, a
/// mixture-of-experts [`Spec`] block, or a recurrent (linear-attention / SSM)
/// block (which takes no `Spec`).
#[derive(Clone, Copy)]
enum Comp {
	Dense(Spec),
	Moe(Spec),
	Recurrent,
	/// Mamba-1 selective-SSM block (norm + selective scan + residual, no FFN):
	/// the shared recurrent mixer for the mamba family, dispatched to
	/// [`layer_mamba`].
	Mamba,
	/// Mamba-2 grouped-SSM (SSD) block (norm + grouped selective scan + gated
	/// grouped RMSNorm + residual, no FFN), dispatched to [`layer_mamba2`].
	Mamba2,
	/// Per-layer attention/recurrent-interleaving block for the mamba hybrids
	/// (jamba, falcon-h1, granitehybrid, nemotron_h): each layer's mixer is
	/// chosen by tensor presence, dispatched to [`layer_hybrid`].
	Hybrid(Hy),
	/// talkie: non-parametric-RMS dense-attention block with post-rope asymmetric
	/// qk-norm and a frozen normed-embedding skip residual, dispatched to
	/// [`layer_talkie`]. Carries a [`Spec`] for the shared scalar resolvers
	/// (logit_scale, embd_skip, final norm).
	Talkie(Spec),
	/// minicpm3: naive (non-absorbed) MLA with LongRoPE per-pair frequency factors
	/// and the minicpm depth-scaled residual, dispatched to [`layer_minicpm3`].
	/// Carries a [`Spec`] for the FFN + the `scale_embd` constant.
	Minicpm3(Spec),
}

/// GGUF architecture string -> decode composition. The single source of truth
/// for both [`SUPPORTED`] and [`dispatch`].
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

/// The architecture strings of [`TABLE`], derived at compile time.
const fn supported_names() -> [&'static str; TABLE.len()] {
	let mut names = [""; TABLE.len()];
	let mut i = 0;
	while i < TABLE.len() {
		names[i] = TABLE[i].0;
		i += 1;
	}
	names
}

/// GGUF architecture strings with a decode composition wired into [`dispatch`].
pub(super) const COMPOSABLE: &[&str] = &supported_names();

/// Architectures whose EVERY parity fixture config matches llama.cpp to
/// NMSE <= 1e-4 (archs_parity). Entry here is by measurement only; the
/// parity test hard-fails if a listed arch regresses.
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

/// True if every parity fixture config of `arch` is measured OK.
pub(super) fn verified(arch: &str) -> bool {
	VERIFIED.contains(&arch)
}

/// True if `arch` has a composition wired into [`dispatch`].
pub(super) fn supported(arch: &str) -> bool {
	COMPOSABLE.contains(&arch)
}

/// True if `arch` is a gated-delta-net hybrid (qwen3.5/next/moe, kimi-linear):
/// its recurrent layers run the delta-rule scan and its dispatch is per-layer
/// [`HyMode::DeltaNet`]. Gates the delta-scratch arena sizing so a non-delta
/// arch pays nothing.
pub(super) fn is_delta_arch(arch: &str) -> bool {
	for &(name, comp) in TABLE {
		if name == arch {
			return matches!(comp, Comp::Hybrid(hy) if hy.mode == HyMode::DeltaNet);
		}
	}
	return false;
}

/// True if `arch`'s block norm is a true LayerNorm (mean-centered). Selects the
/// norm-epsilon KV source at load: `attention.layer_norm_epsilon` for LayerNorm
/// arches, `attention.layer_norm_rms_epsilon` for RMS arches, mirroring
/// llama.cpp's `f_norm_eps` vs `f_norm_rms_eps` split. Fixtures ship the unused
/// key as 0, so reading the wrong one silently zeroes eps.
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

/// The [`Spec`] for `m.hp.arch`, or `None` for recurrent / unlisted arches. Lets
/// the neutral runtime resolve a Spec-flagged scalar without per-arch branching.
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

/// Input-embedding scale for `m.hp.arch`: `sqrt(n_embd)` for arches that hardcode
/// it (gemma family), the `{arch}.embedding_scale` KV for arches that read it
/// (grok, minicpm), else `1.0`. A structural [`Spec`] flag picks the source, so a
/// KV value that every fixture ships never triggers on its own.
pub(super) fn embedding_scale(m: &Model) -> f64 {
	match spec_of(m) {
		Some(sp) if sp.emb_sqrt_ne => (m.hp.ne as f64).sqrt(),
		Some(sp) if sp.emb_scale_kv && m.hp.embedding_scale > 0.0 => m.hp.embedding_scale,
		Some(sp) if sp.emb_scale_const != 1.0 => sp.emb_scale_const,
		_other => 1.0,
	}
}

/// True if `m.hp.arch` adds a learned LM-head output bias to the logits (qwen2,
/// phi2/phi3, pangu). Spec-flag gated, never presence: dream/qwen2vl ship the
/// tensor but their reference graph leaves it unused.
pub(super) fn out_bias(m: &Model) -> bool {
	spec_of(m).is_some_and(|sp| sp.out_bias)
}

/// Final-logit softcap for `m.hp.arch`: `hp.softcap` for arches whose graph applies
/// it (gemma2/gemma3), else `0.0`. Spec-flag gated, never KV-triggered, because
/// gemma1/minicpm ship `final_logit_softcapping` in KV yet must not apply it.
pub(super) fn final_softcap(m: &Model) -> f64 {
	match spec_of(m) {
		Some(sp) if sp.final_softcap => m.hp.softcap,
		_other => 0.0,
	}
}

/// True if `m.hp.arch` retains a non-parametric-normed copy of the initial
/// embedding as a per-layer skip residual (talkie): the decode loop RMS-norms the
/// embedding in place and stashes it, and [`layer_talkie`] adds it back scaled.
pub(super) fn embd_skip(m: &Model) -> bool {
	return spec_of(m).is_some_and(|sp| sp.embd_skip);
}

/// Final-logit scale for `m.hp.arch`: `hp.logit_scale` for arches whose graph
/// scales the logits (command-r), else `1.0`. Spec-flag gated so an arch that
/// ships the KV without using it never scales.
pub(super) fn logit_scale(m: &Model) -> f64 {
	match spec_of(m) {
		Some(sp) if sp.logit_scale && m.hp.logit_scale != 0.0 => m.hp.logit_scale,
		_other => 1.0,
	}
}

/// The final pre-LM-head norm for `m.hp.arch`, composed from the arch's [`NormK`]
/// with gamma/beta resolved by presence: absent gamma is the non-parametric case
/// (olmo/talkie), a present gamma with absent beta the gamma-only case
/// (command-r/dbrx). Mirrors llama.cpp's `build_norm(cur, output_norm_or_null,
/// output_norm_b_or_null, kind, -1)`, so the runtime composes the head norm the
/// same way it composes the block norms.
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

/// Route one decode layer to the ported composition for `m.hp.arch` via a
/// [`TABLE`] lookup, composing through the shared `common` drivers.
pub(super) fn dispatch(
	m: &Model,
	l: usize,
	h_in: &GpuBuffer,
	h_out: &GpuBuffer,
	t: usize,
	ar: &Arena,
	attn_scale: &GpuBuffer,
) -> Result<()> {
	let arch = m.hp.arch.as_str();
	for &(name, comp) in TABLE {
		if name == arch {
			return match comp {
				Comp::Dense(sp) | Comp::Moe(sp) if m.layer_is_moe(l) => {
					layer_moe(m, l, &sp, h_in, h_out, t, ar, attn_scale)
				}
				Comp::Dense(sp) | Comp::Moe(sp) => {
					layer_spec(m, l, &sp, h_in, h_out, t, ar, attn_scale)
				}
				Comp::Recurrent => layer_recurrent(m, l, h_in, h_out, t, ar, attn_scale),
				Comp::Mamba => layer_mamba(m, l, h_in, h_out, t, ar),
				Comp::Mamba2 => layer_mamba2(m, l, h_in, h_out, t, ar),
				Comp::Hybrid(hy) => layer_hybrid(m, l, &hy, h_in, h_out, t, ar, attn_scale),
				Comp::Talkie(sp) => layer_talkie(m, l, &sp, h_in, h_out, t, ar, attn_scale),
				Comp::Minicpm3(sp) => layer_minicpm3(m, l, &sp, h_in, h_out, t, ar, attn_scale),
			};
		}
	}
	bail!(
		"unsupported architecture {arch:?}: no decode composition in models/; \
		 add its TABLE entry"
	)
}
