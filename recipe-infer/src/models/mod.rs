//! Flat declarative arch table: each GGUF `general.architecture` string maps to
//! a [`Comp`] composition entry, and [`dispatch`] is a table lookup that composes
//! the block via the shared `common` drivers. The runtime composes the arch from
//! shared ops and transfers; it does not implement any arch imperatively.
mod common;

use super::{Arena, Model};
use anyhow::{Result, bail};
use common::{Ffn, Spec, layer_moe, layer_recurrent, layer_spec};
use gpu_core::memory::GpuBuffer;

/// One architecture's decode composition: a dense [`Spec`] block, a
/// mixture-of-experts [`Spec`] block, or a recurrent (linear-attention / SSM)
/// block (which takes no `Spec`).
#[derive(Clone, Copy)]
enum Comp {
	Dense(Spec),
	Moe(Spec),
	Recurrent,
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
	("bloom", Comp::Dense(Spec::dense(Ffn::GeluSeq).layer().bias())),
	("chameleon", Comp::Dense(Spec::dense(Ffn::SiluGate).qk())),
	("chatglm", Comp::Dense(Spec::dense(Ffn::SiluGate).bias())),
	("codeshell", Comp::Dense(Spec::dense(Ffn::GeluSeq).layer().bias())),
	("cogvlm", Comp::Dense(Spec::dense(Ffn::SiluGate))),
	("cohere2", Comp::Dense(Spec::dense(Ffn::SiluGate).sandwich())),
	("cohere2moe", Comp::Moe(Spec::dense(Ffn::SiluGate))),
	("command-r", Comp::Dense(Spec::dense(Ffn::SiluGate).sandwich())),
	("dbrx", Comp::Moe(Spec::dense(Ffn::SiluGate))),
	("deci", Comp::Dense(Spec::dense(Ffn::SiluGate).bias())),
	("deepseek", Comp::Moe(Spec::dense(Ffn::SiluGate))),
	("deepseek2", Comp::Moe(Spec::dense(Ffn::SiluGate))),
	("deepseek2-ocr", Comp::Dense(Spec::dense(Ffn::SiluGate).encoder())),
	("deepseek32", Comp::Moe(Spec::dense(Ffn::SiluGate))),
	("deepseek4", Comp::Moe(Spec::dense(Ffn::SiluGate))),
	("dflash", Comp::Dense(Spec::dense(Ffn::SiluGate).qk())),
	("dots1", Comp::Moe(Spec::dense(Ffn::SiluGate).qk())),
	("dream", Comp::Dense(Spec::dense(Ffn::SiluGate))),
	("eagle3", Comp::Dense(Spec::dense(Ffn::SiluGate))),
	("ernie4_5", Comp::Dense(Spec::dense(Ffn::SiluGate))),
	("ernie4_5-moe", Comp::Moe(Spec::dense(Ffn::SiluGate))),
	("eurobert", Comp::Dense(Spec::dense(Ffn::SiluGate).encoder())),
	("exaone", Comp::Dense(Spec::dense(Ffn::SiluGate).bias())),
	("exaone4", Comp::Dense(Spec::dense(Ffn::SiluGate).sandwich())),
	("exaone-moe", Comp::Moe(Spec::dense(Ffn::SiluGate).qk())),
	("falcon", Comp::Dense(Spec::dense(Ffn::GeluSeq).layer().bias())),
	("falcon-h1", Comp::Recurrent),
	("gemma", Comp::Dense(Spec::dense(Ffn::GeluGate))),
	("gemma2", Comp::Dense(Spec::dense(Ffn::GeluGate).sandwich())),
	("gemma3", Comp::Dense(Spec::dense(Ffn::GeluGate).sandwich().qk())),
	("gemma3n", Comp::Dense(Spec::dense(Ffn::GeluGate).sandwich().qk())),
	("gemma4", Comp::Dense(Spec::dense(Ffn::GeluGate).sandwich().qk())),
	("gemma4-assistant", Comp::Dense(Spec::dense(Ffn::GeluGate).sandwich().qk())),
	("gemma-embedding", Comp::Dense(Spec::dense(Ffn::GeluGate).sandwich().qk())),
	("glm4", Comp::Dense(Spec::dense(Ffn::SiluGate).sandwich())),
	("glm4moe", Comp::Moe(Spec::dense(Ffn::SiluGate).qk())),
	("glm-dsa", Comp::Moe(Spec::dense(Ffn::SiluGate))),
	("gpt2", Comp::Dense(Spec::dense(Ffn::GeluSeq).layer().bias())),
	("gptj", Comp::Dense(Spec::dense(Ffn::GeluSeq))),
	("gptneox", Comp::Dense(Spec::dense(Ffn::GeluSeq).layer().bias())),
	("gpt-oss", Comp::Moe(Spec::dense(Ffn::SiluGate))),
	("granite", Comp::Dense(Spec::dense(Ffn::SiluGate))),
	("granitehybrid", Comp::Recurrent),
	("granitemoe", Comp::Moe(Spec::dense(Ffn::SiluGate))),
	("grok", Comp::Moe(Spec::dense(Ffn::SiluGate))),
	("grovemoe", Comp::Moe(Spec::dense(Ffn::SiluGate).qk())),
	("hunyuan-dense", Comp::Dense(Spec::dense(Ffn::SiluGate))),
	("hunyuan-moe", Comp::Moe(Spec::dense(Ffn::SiluGate).qk())),
	("hunyuan_vl", Comp::Dense(Spec::dense(Ffn::SiluGate).qk())),
	("hy_v3", Comp::Moe(Spec::dense(Ffn::SiluGate).qk())),
	("internlm2", Comp::Dense(Spec::dense(Ffn::SiluGate))),
	("jais", Comp::Dense(Spec::dense(Ffn::GeluSeq).layer().bias())),
	("jais2", Comp::Dense(Spec::dense(Ffn::ReluSqrSeq).bias())),
	("jamba", Comp::Recurrent),
	("jina-bert-v2", Comp::Dense(Spec::dense(Ffn::GeluSeq).layer().encoder())),
	("jina-bert-v3", Comp::Dense(Spec::dense(Ffn::GeluSeq).encoder())),
	("kimi-linear", Comp::Recurrent),
	("lfm2", Comp::Recurrent),
	("lfm2moe", Comp::Recurrent),
	("llada", Comp::Dense(Spec::dense(Ffn::SiluGate))),
	("llada-moe", Comp::Moe(Spec::dense(Ffn::SiluGate).qk())),
	("llama", Comp::Dense(Spec::dense(Ffn::SiluGate))),
	("llama4", Comp::Moe(Spec::dense(Ffn::SiluGate))),
	("llama-embed", Comp::Dense(Spec::dense(Ffn::SiluGate).encoder())),
	("maincoder", Comp::Dense(Spec::dense(Ffn::SiluGate).qk())),
	("mamba", Comp::Recurrent),
	("mamba2", Comp::Recurrent),
	("mellum", Comp::Moe(Spec::dense(Ffn::SiluGate).qk())),
	("mimo2", Comp::Moe(Spec::dense(Ffn::SiluGate))),
	("minicpm", Comp::Dense(Spec::dense(Ffn::SiluGate))),
	("minicpm3", Comp::Dense(Spec::dense(Ffn::SiluGate))),
	("minimax-m2", Comp::Moe(Spec::dense(Ffn::SiluGate).qk())),
	("mistral3", Comp::Dense(Spec::dense(Ffn::SiluGate))),
	("mistral4", Comp::Dense(Spec::dense(Ffn::SiluGate))),
	("modern-bert", Comp::Dense(Spec::dense(Ffn::GeluGate).encoder())),
	("mpt", Comp::Dense(Spec::dense(Ffn::GeluSeq).layer().bias())),
	("nemotron", Comp::Dense(Spec::dense(Ffn::ReluSqrSeq).bias())),
	("nemotron_h", Comp::Recurrent),
	("nemotron_h_moe", Comp::Recurrent),
	("neo-bert", Comp::Dense(Spec::dense(Ffn::SiluGate).encoder())),
	("nomic-bert", Comp::Dense(Spec::dense(Ffn::GeluSeq).encoder())),
	("nomic-bert-moe", Comp::Dense(Spec::dense(Ffn::GeluSeq).encoder())),
	("olmo", Comp::Dense(Spec::dense(Ffn::SiluGate))),
	("olmo2", Comp::Dense(Spec::dense(Ffn::SiluGate).sandwich())),
	("olmoe", Comp::Moe(Spec::dense(Ffn::SiluGate).qk())),
	("openelm", Comp::Dense(Spec::dense(Ffn::SiluGate).qk())),
	("orion", Comp::Dense(Spec::dense(Ffn::GeluSeq).layer().bias())),
	("paddleocr", Comp::Dense(Spec::dense(Ffn::SiluGate))),
	("pangu-embedded", Comp::Dense(Spec::dense(Ffn::SiluGate))),
	("phi2", Comp::Dense(Spec::dense(Ffn::GeluSeq).layer().bias())),
	("phi3", Comp::Moe(Spec::dense(Ffn::SiluGate))),
	("phimoe", Comp::Moe(Spec::dense(Ffn::SiluGate))),
	("plamo", Comp::Dense(Spec::dense(Ffn::SiluGate))),
	("plamo2", Comp::Recurrent),
	("plamo3", Comp::Dense(Spec::dense(Ffn::SiluGate).qk().sandwich())),
	("plm", Comp::Dense(Spec::dense(Ffn::ReluSqrSeq).bias())),
	("qwen", Comp::Dense(Spec::dense(Ffn::SiluGate))),
	("qwen2", Comp::Dense(Spec::dense(Ffn::SiluGate).bias())),
	("qwen2moe", Comp::Moe(Spec::dense(Ffn::SiluGate))),
	("qwen2vl", Comp::Dense(Spec::dense(Ffn::SiluGate).bias())),
	("qwen3", Comp::Dense(Spec::dense(Ffn::SiluGate).qk())),
	("qwen35", Comp::Recurrent),
	("qwen35moe", Comp::Recurrent),
	("qwen3moe", Comp::Moe(Spec::dense(Ffn::SiluGate).qk())),
	("qwen3next", Comp::Recurrent),
	("qwen3vl", Comp::Dense(Spec::dense(Ffn::SiluGate).qk())),
	("qwen3vlmoe", Comp::Moe(Spec::dense(Ffn::SiluGate).qk())),
	("refact", Comp::Dense(Spec::dense(Ffn::GeluSeq).layer().bias())),
	("rnd1", Comp::Moe(Spec::dense(Ffn::SiluGate).qk())),
	("rwkv6", Comp::Recurrent),
	("rwkv6qwen2", Comp::Recurrent),
	("rwkv7", Comp::Recurrent),
	("seed_oss", Comp::Dense(Spec::dense(Ffn::SiluGate).sandwich())),
	("smallthinker", Comp::Moe(Spec::dense(Ffn::SiluGate))),
	("smollm3", Comp::Dense(Spec::dense(Ffn::SiluGate))),
	("stablelm", Comp::Dense(Spec::dense(Ffn::SiluGate).qk())),
	("starcoder", Comp::Dense(Spec::dense(Ffn::GeluSeq).layer().bias())),
	("starcoder2", Comp::Dense(Spec::dense(Ffn::GeluSeq).layer().bias())),
	("step35", Comp::Moe(Spec::dense(Ffn::SiluGate).qk())),
	("t5", Comp::Dense(Spec::dense(Ffn::GeluSeq).encoder())),
	("t5encoder", Comp::Dense(Spec::dense(Ffn::GeluSeq).encoder())),
	("talkie", Comp::Dense(Spec::dense(Ffn::SiluGate).qk())),
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
pub(super) const SUPPORTED: &[&str] = &supported_names();

/// True if `arch` has a composition wired into [`dispatch`].
pub(super) fn supported(arch: &str) -> bool {
	SUPPORTED.contains(&arch)
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
			};
		}
	}
	bail!(
		"unsupported architecture {arch:?}: no decode composition in models/; \
		 add its TABLE entry"
	)
}
