//! One module per architecture in llama.cpp/src/models/. `dispatch` maps the
//! GGUF `general.architecture` string to that arch's decode composition.
mod common;
mod afmoe;
mod apertus;
mod arcee;
mod arctic;
mod arwkv7;
mod baichuan;
mod bailingmoe;
mod bailingmoe2;
mod bert;
mod bitnet;
mod bloom;
mod chameleon;
mod chatglm;
mod codeshell;
mod cogvlm;
mod cohere2;
mod cohere2moe;
mod command_r;
mod dbrx;
mod deci;
mod deepseek;
mod deepseek2;
mod deepseek2_ocr;
mod deepseek32;
mod deepseek4;
mod dflash;
mod dots1;
mod dream;
mod eagle3;
mod ernie4_5;
mod ernie4_5_moe;
mod eurobert;
mod exaone;
mod exaone4;
mod exaone_moe;
mod falcon;
mod falcon_h1;
mod gemma;
mod gemma2;
mod gemma3;
mod gemma3n;
mod gemma4;
mod gemma4_assistant;
mod gemma_embedding;
mod glm4;
mod glm4moe;
mod glm_dsa;
mod gpt2;
mod gptj;
mod gptneox;
mod gpt_oss;
mod granite;
mod granitehybrid;
mod granitemoe;
mod grok;
mod grovemoe;
mod hunyuan_dense;
mod hunyuan_moe;
mod hunyuan_vl;
mod hy_v3;
mod internlm2;
mod jais;
mod jais2;
mod jamba;
mod jina_bert_v2;
mod jina_bert_v3;
mod kimi_linear;
mod lfm2;
mod lfm2moe;
mod llada;
mod llada_moe;
mod llama;
mod llama4;
mod llama_embed;
mod maincoder;
mod mamba;
mod mamba2;
mod mellum;
mod mimo2;
mod minicpm;
mod minicpm3;
mod minimax_m2;
mod mistral3;
mod mistral4;
mod modern_bert;
mod mpt;
mod nemotron;
mod nemotron_h;
mod nemotron_h_moe;
mod neo_bert;
mod nomic_bert;
mod nomic_bert_moe;
mod olmo;
mod olmo2;
mod olmoe;
mod openelm;
mod orion;
mod paddleocr;
mod pangu_embedded;
mod phi2;
mod phi3;
mod phimoe;
mod plamo;
mod plamo2;
mod plamo3;
mod plm;
mod qwen;
mod qwen2;
mod qwen2moe;
mod qwen2vl;
mod qwen3;
mod qwen35;
mod qwen35moe;
mod qwen3moe;
mod qwen3next;
mod qwen3vl;
mod qwen3vlmoe;
mod refact;
mod rnd1;
mod rwkv6;
mod rwkv6qwen2;
mod rwkv7;
mod seed_oss;
mod smallthinker;
mod smollm3;
mod stablelm;
mod starcoder;
mod starcoder2;
mod step35;
mod t5;
mod t5encoder;
mod talkie;
mod wavtokenizer_dec;
mod xverse;

use super::{Arena, Model};
use anyhow::{Result, bail};
use gpu_core::memory::GpuBuffer;

/// GGUF architecture strings with a decode composition wired into [`dispatch`].
pub(super) const SUPPORTED: &[&str] = &[
	"afmoe", "apertus", "arcee", "arctic", "arwkv7", "baichuan",
	"bailingmoe", "bailingmoe2", "bert", "bitnet", "bloom", "chameleon",
	"chatglm", "codeshell", "cogvlm", "cohere2", "cohere2moe", "command-r",
	"dbrx", "deci", "deepseek", "deepseek2", "deepseek2-ocr", "deepseek32",
	"deepseek4", "dflash", "dots1", "dream", "eagle3", "ernie4_5",
	"ernie4_5-moe", "eurobert", "exaone", "exaone4", "exaone-moe", "falcon",
	"falcon-h1", "gemma", "gemma2", "gemma3", "gemma3n", "gemma4",
	"gemma4-assistant", "gemma-embedding", "glm4", "glm4moe", "glm-dsa", "gpt2",
	"gptj", "gptneox", "gpt-oss", "granite", "granitehybrid", "granitemoe",
	"grok", "grovemoe", "hunyuan-dense", "hunyuan-moe", "hunyuan_vl", "hy_v3",
	"internlm2", "jais", "jais2", "jamba", "jina-bert-v2", "jina-bert-v3",
	"kimi-linear", "lfm2", "lfm2moe", "llada", "llada-moe", "llama",
	"llama4", "llama-embed", "maincoder", "mamba", "mamba2", "mellum",
	"mimo2", "minicpm", "minicpm3", "minimax-m2", "mistral3", "mistral4",
	"modern-bert", "mpt", "nemotron", "nemotron_h", "nemotron_h_moe", "neo-bert",
	"nomic-bert", "nomic-bert-moe", "olmo", "olmo2", "olmoe", "openelm",
	"orion", "paddleocr", "pangu-embedded", "phi2", "phi3", "phimoe",
	"plamo", "plamo2", "plamo3", "plm", "qwen", "qwen2",
	"qwen2moe", "qwen2vl", "qwen3", "qwen35", "qwen35moe", "qwen3moe",
	"qwen3next", "qwen3vl", "qwen3vlmoe", "refact", "rnd1", "rwkv6",
	"rwkv6qwen2", "rwkv7", "seed_oss", "smallthinker", "smollm3", "stablelm",
	"starcoder", "starcoder2", "step35", "t5", "t5encoder", "talkie",
	"wavtokenizer-dec", "xverse",
];

/// True if `arch` has a composition wired into [`dispatch`].
pub(super) fn supported(arch: &str) -> bool {
	SUPPORTED.contains(&arch)
}

/// Route one decode layer to the ported composition for `m.hp.arch`.
pub(super) fn dispatch(
	m: &Model,
	l: usize,
	h_in: &GpuBuffer,
	h_out: &GpuBuffer,
	t: usize,
	ar: &Arena,
	attn_scale: &GpuBuffer,
) -> Result<()> {
	match m.hp.arch.as_str() {
		"afmoe" => afmoe::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"apertus" => apertus::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"arcee" => arcee::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"arctic" => arctic::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"arwkv7" => arwkv7::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"baichuan" => baichuan::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"bailingmoe" => bailingmoe::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"bailingmoe2" => bailingmoe2::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"bert" => bert::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"bitnet" => bitnet::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"bloom" => bloom::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"chameleon" => chameleon::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"chatglm" => chatglm::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"codeshell" => codeshell::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"cogvlm" => cogvlm::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"cohere2" => cohere2::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"cohere2moe" => cohere2moe::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"command-r" => command_r::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"dbrx" => dbrx::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"deci" => deci::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"deepseek" => deepseek::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"deepseek2" => deepseek2::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"deepseek2-ocr" => deepseek2_ocr::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"deepseek32" => deepseek32::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"deepseek4" => deepseek4::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"dflash" => dflash::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"dots1" => dots1::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"dream" => dream::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"eagle3" => eagle3::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"ernie4_5" => ernie4_5::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"ernie4_5-moe" => ernie4_5_moe::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"eurobert" => eurobert::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"exaone" => exaone::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"exaone4" => exaone4::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"exaone-moe" => exaone_moe::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"falcon" => falcon::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"falcon-h1" => falcon_h1::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"gemma" => gemma::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"gemma2" => gemma2::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"gemma3" => gemma3::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"gemma3n" => gemma3n::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"gemma4" => gemma4::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"gemma4-assistant" => gemma4_assistant::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"gemma-embedding" => gemma_embedding::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"glm4" => glm4::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"glm4moe" => glm4moe::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"glm-dsa" => glm_dsa::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"gpt2" => gpt2::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"gptj" => gptj::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"gptneox" => gptneox::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"gpt-oss" => gpt_oss::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"granite" => granite::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"granitehybrid" => granitehybrid::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"granitemoe" => granitemoe::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"grok" => grok::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"grovemoe" => grovemoe::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"hunyuan-dense" => hunyuan_dense::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"hunyuan-moe" => hunyuan_moe::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"hunyuan_vl" => hunyuan_vl::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"hy_v3" => hy_v3::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"internlm2" => internlm2::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"jais" => jais::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"jais2" => jais2::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"jamba" => jamba::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"jina-bert-v2" => jina_bert_v2::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"jina-bert-v3" => jina_bert_v3::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"kimi-linear" => kimi_linear::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"lfm2" => lfm2::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"lfm2moe" => lfm2moe::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"llada" => llada::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"llada-moe" => llada_moe::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"llama" => llama::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"llama4" => llama4::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"llama-embed" => llama_embed::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"maincoder" => maincoder::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"mamba" => mamba::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"mamba2" => mamba2::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"mellum" => mellum::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"mimo2" => mimo2::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"minicpm" => minicpm::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"minicpm3" => minicpm3::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"minimax-m2" => minimax_m2::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"mistral3" => mistral3::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"mistral4" => mistral4::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"modern-bert" => modern_bert::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"mpt" => mpt::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"nemotron" => nemotron::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"nemotron_h" => nemotron_h::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"nemotron_h_moe" => nemotron_h_moe::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"neo-bert" => neo_bert::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"nomic-bert" => nomic_bert::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"nomic-bert-moe" => nomic_bert_moe::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"olmo" => olmo::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"olmo2" => olmo2::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"olmoe" => olmoe::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"openelm" => openelm::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"orion" => orion::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"paddleocr" => paddleocr::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"pangu-embedded" => pangu_embedded::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"phi2" => phi2::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"phi3" => phi3::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"phimoe" => phimoe::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"plamo" => plamo::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"plamo2" => plamo2::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"plamo3" => plamo3::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"plm" => plm::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"qwen" => qwen::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"qwen2" => qwen2::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"qwen2moe" => qwen2moe::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"qwen2vl" => qwen2vl::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"qwen3" => qwen3::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"qwen35" => qwen35::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"qwen35moe" => qwen35moe::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"qwen3moe" => qwen3moe::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"qwen3next" => qwen3next::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"qwen3vl" => qwen3vl::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"qwen3vlmoe" => qwen3vlmoe::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"refact" => refact::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"rnd1" => rnd1::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"rwkv6" => rwkv6::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"rwkv6qwen2" => rwkv6qwen2::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"rwkv7" => rwkv7::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"seed_oss" => seed_oss::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"smallthinker" => smallthinker::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"smollm3" => smollm3::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"stablelm" => stablelm::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"starcoder" => starcoder::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"starcoder2" => starcoder2::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"step35" => step35::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"t5" => t5::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"t5encoder" => t5encoder::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"talkie" => talkie::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"wavtokenizer-dec" => wavtokenizer_dec::layer(m, l, h_in, h_out, t, ar, attn_scale),
		"xverse" => xverse::layer(m, l, h_in, h_out, t, ar, attn_scale),
		other => bail!(
			"unsupported architecture {other:?}: no decode composition in models/; \
			 add its module and dispatch arm"
		),
	}
}
