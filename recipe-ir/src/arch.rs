#[derive(Clone, Copy, PartialEq)]
pub enum NormK {
	Rms,
	Layer,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Ffn {
	SiluGate,
	GeluGate,
	GeluSeq,
	ReluSqrSeq,
}

#[derive(Clone, Copy)]
pub struct Spec {
	pub norm: NormK,
	pub qk_norm: bool,
	pub post_attn: bool,
	pub post_ffn: bool,
	pub parallel: bool,
	pub pre_norm: bool,
	pub nonparam: bool,
	pub second_attn_norm: bool,
	pub attn_bias: bool,
	pub o_bias: bool,
	pub ffn_bias: bool,
	pub out_bias: bool,
	pub logit_scale: bool,
	pub alibi: bool,
	pub emb_sqrt_ne: bool,
	pub emb_scale_kv: bool,
	pub emb_scale_const: f64,
	pub embd_skip: bool,
	pub residual_scale: bool,
	pub final_softcap: bool,
	pub bidir: bool,
	pub rope: bool,
	pub mla: bool,
	pub ffn: Ffn,
}

impl Spec {
	pub const fn dense(ffn: Ffn) -> Spec {
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
	pub const fn encoder(mut self) -> Spec {
		self.bidir = true;
		self.rope = false;
		self
	}
	pub const fn qk(mut self) -> Spec {
		self.qk_norm = true;
		self
	}
	pub const fn sandwich(mut self) -> Spec {
		self.post_attn = true;
		self.post_ffn = true;
		self
	}
	pub const fn parallel(mut self) -> Spec {
		self.parallel = true;
		self
	}
	pub const fn nonparam(mut self) -> Spec {
		self.nonparam = true;
		self
	}
	pub const fn no_pre_norm(mut self) -> Spec {
		self.pre_norm = false;
		self
	}
	pub const fn attn_norm2(mut self) -> Spec {
		self.second_attn_norm = true;
		self
	}
	pub const fn logit_scale(mut self) -> Spec {
		self.logit_scale = true;
		self
	}
	pub const fn bias(mut self) -> Spec {
		self.attn_bias = true;
		self
	}
	pub const fn o_bias(mut self) -> Spec {
		self.o_bias = true;
		self
	}
	pub const fn ffn_bias(mut self) -> Spec {
		self.ffn_bias = true;
		self
	}
	pub const fn out_bias(mut self) -> Spec {
		self.out_bias = true;
		self
	}
	pub const fn emb_sqrt_ne(mut self) -> Spec {
		self.emb_sqrt_ne = true;
		self
	}
	pub const fn emb_scale_kv(mut self) -> Spec {
		self.emb_scale_kv = true;
		self
	}
	pub const fn emb_scale_const(mut self, v: f64) -> Spec {
		self.emb_scale_const = v;
		self
	}
	pub const fn embd_skip(mut self) -> Spec {
		self.embd_skip = true;
		self
	}
	pub const fn residual_scale(mut self) -> Spec {
		self.residual_scale = true;
		self
	}
	pub const fn final_softcap(mut self) -> Spec {
		self.final_softcap = true;
		self
	}
	pub const fn layer(mut self) -> Spec {
		self.norm = NormK::Layer;
		self
	}
	pub const fn learned_pos(mut self) -> Spec {
		self.rope = false;
		self
	}
	pub const fn alibi(mut self) -> Spec {
		self.alibi = true;
		self.rope = false;
		self
	}
	pub const fn mla(mut self) -> Spec {
		self.mla = true;
		self
	}
	pub const fn no_rope(mut self) -> Spec {
		self.rope = false;
		self
	}
}

#[derive(Clone, Copy, PartialEq)]
pub enum Recur {
	Mamba1,
	Mamba2,
	Plamo2,
	GatedDelta,
	Kda,
	ShortConv,
}

#[derive(Clone, Copy, PartialEq)]
pub enum HyMode {
	MixerFfn,
	Parallel,
	Triage,
	Sandwich,
	DeltaNet,
	ShortConv,
}

#[derive(Clone, Copy)]
pub struct Hy {
	pub recur: Recur,
	pub sp: Spec,
	pub mode: HyMode,
}

#[derive(Clone, Copy)]
pub enum Comp {
	Dense(Spec),
	Moe(Spec),
	Recurrent,
	Mamba,
	Mamba2,
	Hybrid(Hy),
	Talkie(Spec),
	Minicpm3(Spec),
}

pub const TABLE: &[(&str, Comp)] = &[
	("afmoe", Comp::Moe(Spec::dense(Ffn::SiluGate).qk())),
	(
		"apertus",
		Comp::Dense(Spec::dense(Ffn::SiluGate).sandwich()),
	),
	("arcee", Comp::Dense(Spec::dense(Ffn::ReluSqrSeq).bias())),
	("arctic", Comp::Moe(Spec::dense(Ffn::SiluGate))),
	("arwkv7", Comp::Recurrent),
	("baichuan", Comp::Dense(Spec::dense(Ffn::SiluGate))),
	("bailingmoe", Comp::Moe(Spec::dense(Ffn::SiluGate))),
	("bailingmoe2", Comp::Moe(Spec::dense(Ffn::SiluGate).qk())),
	(
		"bert",
		Comp::Dense(Spec::dense(Ffn::GeluSeq).layer().encoder()),
	),
	("bitnet", Comp::Dense(Spec::dense(Ffn::SiluGate))),
	(
		"bloom",
		Comp::Dense(
			Spec::dense(Ffn::GeluSeq)
				.layer()
				.bias()
				.o_bias()
				.alibi()
				.ffn_bias(),
		),
	),
	("chameleon", Comp::Dense(Spec::dense(Ffn::SiluGate).qk())),
	("chatglm", Comp::Dense(Spec::dense(Ffn::SiluGate).bias())),
	(
		"codeshell",
		Comp::Dense(Spec::dense(Ffn::GeluSeq).layer().bias().o_bias().ffn_bias()),
	),
	("cogvlm", Comp::Dense(Spec::dense(Ffn::SiluGate))),
	(
		"cohere2",
		Comp::Dense(Spec::dense(Ffn::SiluGate).sandwich()),
	),
	("cohere2moe", Comp::Moe(Spec::dense(Ffn::SiluGate))),
	(
		"command-r",
		Comp::Dense(Spec::dense(Ffn::SiluGate).layer().parallel().logit_scale()),
	),
	("dbrx", Comp::Moe(Spec::dense(Ffn::SiluGate).layer())),
	(
		"deci",
		Comp::Dense(Spec::dense(Ffn::SiluGate).o_bias().ffn_bias()),
	),
	("deepseek", Comp::Moe(Spec::dense(Ffn::SiluGate))),
	("deepseek2", Comp::Moe(Spec::dense(Ffn::SiluGate).mla())),
	(
		"deepseek2-ocr",
		Comp::Dense(Spec::dense(Ffn::SiluGate).encoder()),
	),
	("deepseek32", Comp::Moe(Spec::dense(Ffn::SiluGate).mla())),
	("deepseek4", Comp::Moe(Spec::dense(Ffn::SiluGate))),
	("dflash", Comp::Dense(Spec::dense(Ffn::SiluGate).qk())),
	("dots1", Comp::Moe(Spec::dense(Ffn::SiluGate).qk())),
	("dream", Comp::Dense(Spec::dense(Ffn::SiluGate))),
	("eagle3", Comp::Dense(Spec::dense(Ffn::SiluGate))),
	("ernie4_5", Comp::Dense(Spec::dense(Ffn::SiluGate))),
	("ernie4_5-moe", Comp::Moe(Spec::dense(Ffn::SiluGate))),
	(
		"eurobert",
		Comp::Dense(Spec::dense(Ffn::SiluGate).encoder()),
	),
	("exaone", Comp::Dense(Spec::dense(Ffn::SiluGate).bias())),
	(
		"exaone4",
		Comp::Dense(Spec::dense(Ffn::SiluGate).qk().sandwich().no_pre_norm()),
	),
	("exaone-moe", Comp::Moe(Spec::dense(Ffn::SiluGate).qk())),
	(
		"falcon",
		Comp::Dense(Spec::dense(Ffn::GeluSeq).layer().parallel().attn_norm2()),
	),
	(
		"falcon-h1",
		Comp::Hybrid(Hy {
			recur: Recur::Mamba2,
			sp: Spec::dense(Ffn::SiluGate).ffn_bias(),
			mode: HyMode::Parallel,
		}),
	),
	(
		"gemma",
		Comp::Dense(Spec::dense(Ffn::GeluGate).emb_sqrt_ne()),
	),
	(
		"gemma2",
		Comp::Dense(
			Spec::dense(Ffn::GeluGate)
				.sandwich()
				.emb_sqrt_ne()
				.final_softcap(),
		),
	),
	(
		"gemma3",
		Comp::Dense(Spec::dense(Ffn::GeluGate).sandwich().qk()),
	),
	(
		"gemma3n",
		Comp::Dense(Spec::dense(Ffn::GeluGate).sandwich().qk()),
	),
	(
		"gemma4",
		Comp::Dense(Spec::dense(Ffn::GeluGate).sandwich().qk()),
	),
	(
		"gemma4-assistant",
		Comp::Dense(Spec::dense(Ffn::GeluGate).sandwich().qk()),
	),
	(
		"gemma-embedding",
		Comp::Dense(Spec::dense(Ffn::GeluGate).sandwich().qk()),
	),
	("glm4", Comp::Dense(Spec::dense(Ffn::SiluGate).sandwich())),
	("glm4moe", Comp::Moe(Spec::dense(Ffn::SiluGate).qk())),
	("glm-dsa", Comp::Moe(Spec::dense(Ffn::SiluGate).mla())),
	(
		"gpt2",
		Comp::Dense(
			Spec::dense(Ffn::GeluSeq)
				.layer()
				.bias()
				.o_bias()
				.learned_pos()
				.ffn_bias(),
		),
	),
	("gptj", Comp::Dense(Spec::dense(Ffn::GeluSeq))),
	(
		"gptneox",
		Comp::Dense(Spec::dense(Ffn::GeluSeq).layer().bias().o_bias().ffn_bias()),
	),
	("gpt-oss", Comp::Moe(Spec::dense(Ffn::SiluGate))),
	(
		"granite",
		Comp::Dense(Spec::dense(Ffn::SiluGate).o_bias().ffn_bias()),
	),
	(
		"granitehybrid",
		Comp::Hybrid(Hy {
			recur: Recur::Mamba2,
			sp: Spec::dense(Ffn::SiluGate).o_bias().ffn_bias(),
			mode: HyMode::MixerFfn,
		}),
	),
	(
		"granitemoe",
		Comp::Moe(Spec::dense(Ffn::SiluGate).o_bias().ffn_bias()),
	),
	("grok", Comp::Moe(Spec::dense(Ffn::SiluGate).emb_scale_kv())),
	("grovemoe", Comp::Moe(Spec::dense(Ffn::SiluGate).qk())),
	("hunyuan-dense", Comp::Dense(Spec::dense(Ffn::SiluGate))),
	("hunyuan-moe", Comp::Moe(Spec::dense(Ffn::SiluGate).qk())),
	("hunyuan_vl", Comp::Dense(Spec::dense(Ffn::SiluGate).qk())),
	("hy_v3", Comp::Moe(Spec::dense(Ffn::SiluGate).qk())),
	("internlm2", Comp::Dense(Spec::dense(Ffn::SiluGate))),
	(
		"jais",
		Comp::Dense(Spec::dense(Ffn::GeluSeq).layer().bias().o_bias().alibi()),
	),
	(
		"jais2",
		Comp::Dense(
			Spec::dense(Ffn::ReluSqrSeq)
				.layer()
				.bias()
				.o_bias()
				.ffn_bias(),
		),
	),
	(
		"jamba",
		Comp::Hybrid(Hy {
			recur: Recur::Mamba1,
			sp: Spec::dense(Ffn::SiluGate).no_rope(),
			mode: HyMode::MixerFfn,
		}),
	),
	(
		"jina-bert-v2",
		Comp::Dense(Spec::dense(Ffn::GeluSeq).layer().encoder()),
	),
	(
		"jina-bert-v3",
		Comp::Dense(Spec::dense(Ffn::GeluSeq).encoder()),
	),
	(
		"kimi-linear",
		Comp::Hybrid(Hy {
			recur: Recur::Kda,
			sp: Spec::dense(Ffn::SiluGate),
			mode: HyMode::DeltaNet,
		}),
	),
	(
		"lfm2",
		Comp::Hybrid(Hy {
			recur: Recur::ShortConv,
			sp: Spec::dense(Ffn::SiluGate).qk(),
			mode: HyMode::ShortConv,
		}),
	),
	(
		"lfm2moe",
		Comp::Hybrid(Hy {
			recur: Recur::ShortConv,
			sp: Spec::dense(Ffn::SiluGate).qk(),
			mode: HyMode::ShortConv,
		}),
	),
	("llada", Comp::Dense(Spec::dense(Ffn::SiluGate))),
	("llada-moe", Comp::Moe(Spec::dense(Ffn::SiluGate).qk())),
	("llama", Comp::Dense(Spec::dense(Ffn::SiluGate).o_bias())),
	("llama4", Comp::Moe(Spec::dense(Ffn::SiluGate))),
	(
		"llama-embed",
		Comp::Dense(Spec::dense(Ffn::SiluGate).encoder()),
	),
	("maincoder", Comp::Dense(Spec::dense(Ffn::SiluGate).qk())),
	("mamba", Comp::Mamba),
	("mamba2", Comp::Mamba2),
	("mellum", Comp::Moe(Spec::dense(Ffn::SiluGate).qk())),
	("mimo2", Comp::Moe(Spec::dense(Ffn::SiluGate))),
	(
		"minicpm",
		Comp::Dense(
			Spec::dense(Ffn::SiluGate)
				.emb_scale_kv()
				.residual_scale()
				.o_bias()
				.ffn_bias(),
		),
	),
	(
		"minicpm3",
		Comp::Minicpm3(Spec::dense(Ffn::SiluGate).emb_scale_const(12.0)),
	),
	("minimax-m2", Comp::Moe(Spec::dense(Ffn::SiluGate).qk())),
	("mistral3", Comp::Dense(Spec::dense(Ffn::SiluGate).o_bias())),
	("mistral4", Comp::Moe(Spec::dense(Ffn::SiluGate).mla())),
	(
		"modern-bert",
		Comp::Dense(Spec::dense(Ffn::GeluGate).encoder()),
	),
	(
		"mpt",
		Comp::Dense(Spec::dense(Ffn::GeluSeq).layer().bias().o_bias().alibi()),
	),
	(
		"nemotron",
		Comp::Dense(
			Spec::dense(Ffn::ReluSqrSeq)
				.layer()
				.bias()
				.o_bias()
				.ffn_bias(),
		),
	),
	(
		"nemotron_h",
		Comp::Hybrid(Hy {
			recur: Recur::Mamba2,
			sp: Spec::dense(Ffn::ReluSqrSeq).o_bias().no_rope(),
			mode: HyMode::Triage,
		}),
	),
	(
		"nemotron_h_moe",
		Comp::Hybrid(Hy {
			recur: Recur::Mamba2,
			sp: Spec::dense(Ffn::ReluSqrSeq).o_bias().no_rope(),
			mode: HyMode::Triage,
		}),
	),
	(
		"neo-bert",
		Comp::Dense(Spec::dense(Ffn::SiluGate).encoder()),
	),
	(
		"nomic-bert",
		Comp::Dense(Spec::dense(Ffn::GeluSeq).encoder()),
	),
	(
		"nomic-bert-moe",
		Comp::Dense(Spec::dense(Ffn::GeluSeq).encoder()),
	),
	(
		"olmo",
		Comp::Dense(Spec::dense(Ffn::SiluGate).layer().nonparam()),
	),
	("olmo2", Comp::Dense(Spec::dense(Ffn::SiluGate).sandwich())),
	("olmoe", Comp::Moe(Spec::dense(Ffn::SiluGate).qk())),
	("openelm", Comp::Dense(Spec::dense(Ffn::SiluGate).qk())),
	(
		"orion",
		Comp::Dense(Spec::dense(Ffn::GeluSeq).layer().bias().ffn_bias()),
	),
	(
		"paddleocr",
		Comp::Dense(Spec::dense(Ffn::SiluGate).o_bias()),
	),
	(
		"pangu-embedded",
		Comp::Dense(Spec::dense(Ffn::SiluGate).o_bias().out_bias()),
	),
	(
		"phi2",
		Comp::Dense(
			Spec::dense(Ffn::GeluSeq)
				.layer()
				.bias()
				.o_bias()
				.out_bias()
				.parallel()
				.ffn_bias(),
		),
	),
	("phi3", Comp::Moe(Spec::dense(Ffn::SiluGate).out_bias())),
	("phimoe", Comp::Moe(Spec::dense(Ffn::SiluGate))),
	("plamo", Comp::Dense(Spec::dense(Ffn::SiluGate).parallel())),
	(
		"plamo2",
		Comp::Hybrid(Hy {
			recur: Recur::Plamo2,
			sp: Spec::dense(Ffn::SiluGate),
			mode: HyMode::Sandwich,
		}),
	),
	(
		"plamo3",
		Comp::Dense(Spec::dense(Ffn::SiluGate).qk().sandwich()),
	),
	("plm", Comp::Dense(Spec::dense(Ffn::ReluSqrSeq).bias())),
	("qwen", Comp::Dense(Spec::dense(Ffn::SiluGate))),
	(
		"qwen2",
		Comp::Dense(Spec::dense(Ffn::SiluGate).bias().out_bias()),
	),
	("qwen2moe", Comp::Moe(Spec::dense(Ffn::SiluGate))),
	("qwen2vl", Comp::Dense(Spec::dense(Ffn::SiluGate).bias())),
	("qwen3", Comp::Dense(Spec::dense(Ffn::SiluGate).qk())),
	(
		"qwen35",
		Comp::Hybrid(Hy {
			recur: Recur::GatedDelta,
			sp: Spec::dense(Ffn::SiluGate),
			mode: HyMode::DeltaNet,
		}),
	),
	(
		"qwen35moe",
		Comp::Hybrid(Hy {
			recur: Recur::GatedDelta,
			sp: Spec::dense(Ffn::SiluGate),
			mode: HyMode::DeltaNet,
		}),
	),
	("qwen3moe", Comp::Moe(Spec::dense(Ffn::SiluGate).qk())),
	(
		"qwen3next",
		Comp::Hybrid(Hy {
			recur: Recur::GatedDelta,
			sp: Spec::dense(Ffn::SiluGate),
			mode: HyMode::DeltaNet,
		}),
	),
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
	(
		"stablelm",
		Comp::Dense(Spec::dense(Ffn::SiluGate).qk().layer()),
	),
	(
		"starcoder",
		Comp::Dense(
			Spec::dense(Ffn::GeluSeq)
				.layer()
				.bias()
				.o_bias()
				.learned_pos()
				.ffn_bias(),
		),
	),
	(
		"starcoder2",
		Comp::Dense(Spec::dense(Ffn::GeluSeq).layer().bias().o_bias().ffn_bias()),
	),
	("step35", Comp::Moe(Spec::dense(Ffn::SiluGate).qk())),
	("t5", Comp::Dense(Spec::dense(Ffn::GeluSeq).encoder())),
	(
		"t5encoder",
		Comp::Dense(Spec::dense(Ffn::GeluSeq).encoder()),
	),
	(
		"talkie",
		Comp::Talkie(Spec::dense(Ffn::SiluGate).logit_scale().embd_skip()),
	),
	(
		"wavtokenizer-dec",
		Comp::Dense(Spec::dense(Ffn::GeluSeq).encoder()),
	),
	("xverse", Comp::Dense(Spec::dense(Ffn::SiluGate))),
];

pub const fn supported_names() -> [&'static str; TABLE.len()] {
	let mut names = [""; TABLE.len()];
	let mut i = 0;
	while i < TABLE.len() {
		names[i] = TABLE[i].0;
		i += 1;
	}
	names
}

pub const COMPOSABLE: &[&str] = &supported_names();

pub const VERIFIED: &[&str] = &[
	"arcee",
	"arctic",
	"baichuan",
	"bailingmoe",
	"bailingmoe2",
	"chatglm",
	"cogvlm",
	"command-r",
	"dbrx",
	"deci",
	"deepseek",
	"deepseek2",
	"deepseek32",
	"dots1",
	"dream",
	"ernie4_5",
	"ernie4_5-moe",
	"exaone",
	"falcon-h1",
	"gemma",
	"gemma2",
	"glm-dsa",
	"glm4moe",
	"granite",
	"granitemoe",
	"granitehybrid",
	"grok",
	"grovemoe",
	"hunyuan-dense",
	"hunyuan-moe",
	"minicpm",
	"hunyuan_vl",
	"hy_v3",
	"internlm2",
	"jamba",
	"kimi-linear",
	"lfm2",
	"lfm2moe",
	"llada",
	"llada-moe",
	"llama4",
	"maincoder",
	"mamba",
	"mamba2",
	"minicpm3",
	"nemotron_h",
	"nemotron_h_moe",
	"minimax-m2",
	"mistral4",
	"olmoe",
	"openelm",
	"paddleocr",
	"pangu-embedded",
	"phi2",
	"phi3",
	"plamo",
	"plamo2",
	"qwen",
	"qwen2",
	"qwen2moe",
	"qwen2vl",
	"qwen3",
	"qwen35",
	"qwen35moe",
	"qwen3moe",
	"qwen3next",
	"qwen3vl",
	"qwen3vlmoe",
	"rnd1",
	"seed_oss",
	"smallthinker",
	"smollm3",
	"talkie",
	"xverse",
];

pub fn verified(arch: &str) -> bool {
	VERIFIED.contains(&arch)
}

pub fn supported(arch: &str) -> bool {
	COMPOSABLE.contains(&arch)
}

pub fn is_delta_arch(arch: &str) -> bool {
	for &(name, comp) in TABLE {
		if name == arch {
			return matches!(comp, Comp::Hybrid(hy) if hy.mode == HyMode::DeltaNet);
		}
	}
	return false;
}

pub fn norm_is_layer(arch: &str) -> bool {
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

pub fn comp_of(arch: &str) -> Option<Comp> {
	for &(name, comp) in TABLE {
		if name == arch {
			return Some(comp);
		}
	}
	return None;
}

pub fn arch_has_recurrence(arch: &str) -> bool {
	return matches!(
		comp_of(arch),
		Some(Comp::Recurrent | Comp::Mamba | Comp::Mamba2 | Comp::Hybrid(_))
	);
}
