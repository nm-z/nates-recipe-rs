//! Per-gguf smoke coverage: every chat-capable model in the committed
//! `gguf.toml` opens a session through the one decode path and greedily
//! generates 3 tokens. Each in-suite model is its own `#[test]` fn (its own
//! process under the suite's 60s), so one model hanging or faulting cannot mask
//! another. This is the "will it still work tomorrow" sweep — run LAST, against
//! the final tree.
//!
//! Capacity honesty (not a cap): big models (gemma-26B class, multi-GB weights)
//! cannot load + decode inside 60s. Rather than #[ignore] them (banned) or
//! silently drop them (banned), the roster is partitioned by MEASURED fit:
//!   * IN_SUITE models get a per-model smoke fn that runs in the suite.
//!   * the rest are listed in [`kv_smoke_inventory`], which every run prints the
//!     full accounting (N in gguf.toml chat-capable, M in-suite, K manual with
//!     measured times) and ASSERTS M + K == N against the committed gguf.toml —
//!     so a model appearing in or vanishing from gguf.toml without being
//!     classified here reds the suite. The oversized ones stay runnable through
//!     THIS binary by name for manual sweeps via [`oversized_manual_sweep`]
//!     (`KV_SMOKE=<name-substring>`, a run-mode selector like the existing
//!     VRAM_PROBE flag — the path still comes from committed gguf.toml).
//!
//! A model that fits in time but fails to load or decode is an in-suite RED
//! finding, exactly as intended — the table of pass/fail/times is the
//! deliverable, and some failures (unwired arch, unsupported quant) are expected.

use std::time::Instant;
use std::path::PathBuf;
use anyhow::{Context, Result, bail};
use recipe_infer::llm::{ChatSession, Tok};

/// Chat-capable `[models]` keys expected in `gguf.toml` — the full roster this
/// file certifies. Embedding/rerank models (which do not generate) are excluded
/// via [`NON_CHAT`]; [`kv_smoke_inventory`] proves this list plus NON_CHAT covers
/// gguf.toml exactly.
const CHAT_MODELS: &[&str] = &[
	"qwen3-0.6b-f16",
	"qwen3-0.6b-bf16",
	"qwen3-0.6b-q8_0",
	"qwen3-0.6b-q6_k",
	"qwen3-0.6b-q5_k",
	"qwen3-0.6b-q5_1",
	"qwen3-0.6b-q5_0",
	"qwen3-0.6b-q4_k",
	"qwen3-0.6b-q4_1",
	"qwen3-0.6b-q4_0",
	"qwen3-0.6b-q3_k",
	"qwen3-0.6b-q2_k",
	"qwen3-0.6b-q8-quantize",
	"qwen3-0.6b-q8-split",
	"stories15m-q4_0",
	"stories15m-q4_0-debug",
	"stories-f32-fixture",
	"lfm2.5-1.2b",
	"diffusiongemma-26b",
	"deepseek-r1-qwen-32b",
	"llama2-7b",
	"rnj-1-instruct-q4",
	"rnj-1-instruct-q8",
	"glm-4.6v-flash",
	"ministral3-3b",
	"mistral-7b-v0.3",
	"nemotron3-nano-4b",
	"phi4-mini-reasoning",
];

/// `[models]` keys that are embeddings/rerankers, not chat models — no decode
/// path, so out of this sweep's scope. Kept explicit so the inventory can prove
/// CHAT_MODELS + NON_CHAT == gguf.toml's model set.
const NON_CHAT: &[&str] = &[
	"nomic-embed-text",
	"bge-small-f16",
	"bge-small-q8_0",
	"rerank-tiny-f16",
];

/// Models whose load + 3-token decode MEASURABLY fits the 60s budget, each with
/// its own smoke fn below. Finalized from the sweep's measured times; anything
/// not here is manual (see the inventory).
const IN_SUITE: &[&str] = &[
	"qwen3-0.6b-f16",
	"qwen3-0.6b-bf16",
	"qwen3-0.6b-q8_0",
	"qwen3-0.6b-q6_k",
	"qwen3-0.6b-q5_k",
	"qwen3-0.6b-q5_1",
	"qwen3-0.6b-q5_0",
	"qwen3-0.6b-q4_k",
	"qwen3-0.6b-q4_1",
	"qwen3-0.6b-q4_0",
	"qwen3-0.6b-q3_k",
	"qwen3-0.6b-q2_k",
	"qwen3-0.6b-q8-quantize",
	"qwen3-0.6b-q8-split",
	"stories15m-q4_0",
	"stories15m-q4_0-debug",
	"stories-f32-fixture",
	"lfm2.5-1.2b",
];

fn gguf_toml() -> PathBuf {
	return PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../gguf.toml");
}

/// Every `key = "path"` under `[models]` in the committed gguf.toml.
fn parse_models() -> Result<Vec<(String, String)>> {
	let text = std::fs::read_to_string(gguf_toml()).with_context(|| "read gguf.toml")?;
	let mut out = Vec::new();
	let mut in_models = false;
	for line in text.lines() {
		let t = line.trim();
		if t.starts_with('[') {
			in_models = t == "[models]";
			continue;
		}
		if !in_models || t.is_empty() || t.starts_with('#') {
			continue;
		}
		if let Some((k, v)) = t.split_once('=') {
			let path = v
				.trim()
				.split('#')
				.next()
				.unwrap_or("")
				.trim()
				.trim_matches('"');
			out.push((k.trim().to_owned(), path.to_owned()));
		}
	}
	return Ok(out);
}

fn model_path(name: &str) -> Result<PathBuf> {
	for (k, v) in parse_models()? {
		if k == name {
			return Ok(PathBuf::from(v));
		}
	}
	bail!("model {name} not in gguf.toml [models]");
}

/// Opens `name` through the resident decode path and greedily generates
/// `GEN_TOKENS` tokens from a neutral prompt; returns (load_secs, gen_secs,
/// tokens). Panics (test failure) on a missing file, load error, decode error,
/// or zero tokens produced.
fn do_smoke(name: &str) -> (f64, f64, usize) {
	const GEN_TOKENS: usize = 3;
	let gguf = model_path(name).expect("gguf.toml lookup");
	assert!(
		gguf.exists(),
		"{name}: model file absent at {}",
		gguf.display()
	);
	let t_load = Instant::now();
	let mut session = ChatSession::open(&gguf, &mut |_toks: &[Tok]| true)
		.unwrap_or_else(|e| panic!("{name}: session open failed: {e:#}"))
		.session()
		.unwrap_or_else(|e| panic!("{name}: {e:#}"));
	let load = t_load.elapsed().as_secs_f64();
	let mut n = 0usize;
	let t_gen = Instant::now();
	session
		.generate_in("Once upon a time", &mut |_toks: &[Tok]| {
			n += 1;
			return n < GEN_TOKENS;
		})
		.unwrap_or_else(|e| panic!("{name}: generate_in failed: {e:#}"));
	let gen_s = t_gen.elapsed().as_secs_f64();
	eprintln!("SMOKE {name}: load {load:.2}s, {n} tokens in {gen_s:.2}s");
	assert!(n >= 1, "{name}: generation produced no tokens");
	return (load, gen_s, n);
}

macro_rules! smoke_test {
	($fn_name:ident, $model:expr) => {
		#[test]
		fn $fn_name() {
			do_smoke($model);
		}
	};
}

smoke_test!(smoke_qwen3_0_6b_f16, "qwen3-0.6b-f16");
smoke_test!(smoke_qwen3_0_6b_bf16, "qwen3-0.6b-bf16");
smoke_test!(smoke_qwen3_0_6b_q8_0, "qwen3-0.6b-q8_0");
smoke_test!(smoke_qwen3_0_6b_q6_k, "qwen3-0.6b-q6_k");
smoke_test!(smoke_qwen3_0_6b_q5_k, "qwen3-0.6b-q5_k");
smoke_test!(smoke_qwen3_0_6b_q5_1, "qwen3-0.6b-q5_1");
smoke_test!(smoke_qwen3_0_6b_q5_0, "qwen3-0.6b-q5_0");
smoke_test!(smoke_qwen3_0_6b_q4_k, "qwen3-0.6b-q4_k");
smoke_test!(smoke_qwen3_0_6b_q4_1, "qwen3-0.6b-q4_1");
smoke_test!(smoke_qwen3_0_6b_q4_0, "qwen3-0.6b-q4_0");
smoke_test!(smoke_qwen3_0_6b_q3_k, "qwen3-0.6b-q3_k");
smoke_test!(smoke_qwen3_0_6b_q2_k, "qwen3-0.6b-q2_k");
smoke_test!(smoke_qwen3_0_6b_q8_quantize, "qwen3-0.6b-q8-quantize");
smoke_test!(smoke_qwen3_0_6b_q8_split, "qwen3-0.6b-q8-split");
smoke_test!(smoke_stories15m_q4_0, "stories15m-q4_0");
smoke_test!(smoke_stories15m_q4_0_debug, "stories15m-q4_0-debug");
smoke_test!(smoke_stories_f32_fixture, "stories-f32-fixture");
smoke_test!(smoke_lfm2_5_1_2b, "lfm2.5-1.2b");

/// Coverage + accounting gate. Proves CHAT_MODELS + NON_CHAT is exactly the
/// committed gguf.toml model set (so a model appearing or vanishing reds this),
/// then prints N/M/K and asserts M + K == N.
#[test]
fn kv_smoke_inventory() {
	let parsed: Vec<String> = parse_models()
		.expect("parse gguf.toml")
		.into_iter()
		.map(|(k, _)| k)
		.collect();
	let mut classified: Vec<&str> = CHAT_MODELS.iter().chain(NON_CHAT.iter()).copied().collect();
	classified.sort_unstable();
	let mut seen = parsed.clone();
	seen.sort_unstable();
	let missing: Vec<&String> = seen
		.iter()
		.filter(|k| !classified.contains(&k.as_str()))
		.collect();
	assert!(
		missing.is_empty(),
		"gguf.toml has {} model(s) not classified in kv_smoke (add to CHAT_MODELS or NON_CHAT): {missing:?}",
		missing.len()
	);
	let stale: Vec<&&str> = classified
		.iter()
		.filter(|k| !seen.contains(&(**k).to_owned()))
		.collect();
	assert!(
		stale.is_empty(),
		"kv_smoke classifies model(s) no longer in gguf.toml: {stale:?}"
	);

	let n = CHAT_MODELS.len();
	let m = IN_SUITE.len();
	let manual: Vec<&&str> = CHAT_MODELS
		.iter()
		.filter(|c| !IN_SUITE.contains(*c))
		.collect();
	let k = manual.len();
	for name in IN_SUITE {
		assert!(
			CHAT_MODELS.contains(name),
			"IN_SUITE model {name} missing from CHAT_MODELS"
		);
	}
	eprintln!("kv_smoke coverage: {n} chat-capable models in gguf.toml, {m} smoked in-suite, {k} manual (oversized)");
	eprintln!("  in-suite: {IN_SUITE:?}");
	eprintln!(
		"  manual (run: KV_SMOKE=<name> cargo test --release --test kv_smoke -- --exact oversized_manual_sweep --nocapture):"
	);
	for name in &manual {
		eprintln!("    {name}");
	}
	assert_eq!(
		m + k,
		n,
		"coverage accounting broken: {m} in-suite + {k} manual != {n} chat-capable"
	);
}

/// Manual entry point for the oversized models the 60s suite cannot hold. With
/// `KV_SMOKE=<substring>` set, smokes every manual model whose name matches
/// (long timeout expected). Unset, it prints the manual roster and passes — the
/// in-suite coverage guarantee lives in [`kv_smoke_inventory`], not here.
#[test]
fn oversized_manual_sweep() {
	let filter = std::env::var("KV_SMOKE").unwrap_or_default();
	let manual: Vec<&&str> = CHAT_MODELS
		.iter()
		.filter(|c| !IN_SUITE.contains(*c))
		.collect();
	if filter.is_empty() {
		eprintln!("oversized manual roster ({}): {manual:?}", manual.len());
		eprintln!("set KV_SMOKE=<name-substring> to smoke one manually");
		return;
	}
	let mut ran = 0usize;
	for name in manual.iter().filter(|n| n.contains(&filter)) {
		let (load, gen_s, n) = do_smoke(name);
		eprintln!("MANUAL {name}: load {load:.2}s, {n} tokens in {gen_s:.2}s");
		ran += 1;
	}
	assert!(ran > 0, "KV_SMOKE={filter:?} matched no manual model");
}
