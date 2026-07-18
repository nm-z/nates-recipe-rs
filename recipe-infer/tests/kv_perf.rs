//! Permanent perf harness for the resident decode path. Loads a real chat gguf
//! (smallest chat-capable model in the repo's `gguf.toml`: qwen3-0.6B Q8_0),
//! prefills a prompt of at least 2k tokens through the one decode path, greedily
//! generates a fixed token count, and prints TTFT and tok/s to stderr. This is
//! the target the nontemporal A/B and before/after measurements run with
//! `--nocapture`; it is a harness, not a benchmark gate, so it asserts only that
//! generation completed — but it is still a real test that fails on breakage
//! (load error, empty generation, or a decode that returns `Err`), and it stays
//! inside the 60s per-test budget.
//!
//! The prompt is a long deterministic passage tokenized to >= 2000 tokens; the
//! generated text is greedy gibberish continuation, which is all a speed probe
//! needs. Model path comes from the committed `gguf.toml`; the test fails loudly
//! if that model is not present on the machine (a perf harness with no model is a
//! broken harness, not a skip).

use std::path::PathBuf;
use std::process;
use std::sync::Once;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use recipe_infer::gguf::Gguf;
use recipe_infer::llm::{ChatSession, Tok};
use recipe_infer::tokenizer::from_gguf;

const MODEL_KEY: &str = "qwen3-0.6b-q8_0";
const MIN_PROMPT_TOKENS: usize = 2000;
// 3 decode steps is enough steady-state tok/s; qwen never stops on its own so any
// count is arbitrary, and each extra token is the most expensive kind on the
// no-cache baseline. TTFT (prefill) is measured separately, excluded from tok/s.
const GEN_TOKENS: usize = 3;

// Scan/recurrent-hybrid contrast: smallest chat-capable scan arch in gguf.toml
// (lfm2 = short-conv + attention hybrid, 1.2B). Same 3-step protocol as dense.
const SCAN_MODEL_KEY: &str = "lfm2.5-1.2b";
const SCAN_GEN_TOKENS: usize = 3;

fn probe_gate() {
	static GATE: Once = Once::new();
	GATE.call_once(|| {
		if let Some(code) = recipe_infer::llm::vram_probe_ask() {
			process::exit(code);
		}
		if let Some(code) = gpu_core::memory::ram_probe_ask() {
			process::exit(code);
		}
	});
}

/// Path for `key` out of the committed `gguf.toml` at the repo root, parsed
/// without a toml dependency: the `key = "value"` line under `[models]`.
fn model_path(key: &str) -> Result<PathBuf> {
	let toml = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../gguf.toml");
	let text = std::fs::read_to_string(&toml).with_context(|| format!("read {}", toml.display()))?;
	for line in text.lines() {
		let trimmed = line.trim_start();
		let Some(rest) = trimmed.strip_prefix(key) else {
			continue;
		};
		let Some((_, value)) = rest.split_once('=') else {
			continue;
		};
		let path = value.trim().trim_matches('"');
		if !path.is_empty() {
			return Ok(PathBuf::from(path));
		}
	}
	bail!("key {key} not found in {}", toml.display());
}

/// A deterministic passage repeated until it tokenizes to at least
/// `MIN_PROMPT_TOKENS` under the model's own tokenizer.
fn long_prompt(gguf: &std::path::Path) -> Result<String> {
	let g = Gguf::open(gguf)?;
	let tk = from_gguf(&g)?;
	let unit = "The rain in Spain falls mainly on the plain, and the cat sat on the mat by the old wooden door. ";
	let mut prompt = String::new();
	let mut tokens = 0usize;
	while tokens < MIN_PROMPT_TOKENS + 200 {
		prompt.push_str(unit);
		let enc = tk
			.encode(prompt.as_str(), false)
			.map_err(|e| anyhow::anyhow!("tokenize: {e}"))?;
		tokens = enc.get_ids().len();
	}
	anyhow::ensure!(
		tokens >= MIN_PROMPT_TOKENS,
		"prompt tokenized to {tokens} tokens, need >= {MIN_PROMPT_TOKENS}"
	);
	eprintln!(
		"kv_perf: prompt is {tokens} tokens ({} bytes)",
		prompt.len()
	);
	return Ok(prompt);
}

/// Shared probe: prefill a >=2k-token prompt for `model_key`, greedily generate
/// `gen_tokens`, print TTFT/tok/s to stderr under `tag`. Asserts only that
/// generation ran (a perf harness, not a numeric gate).
fn measure(model_key: &str, gen_tokens: usize, tag: &str) {
	probe_gate();
	let gguf = model_path(model_key).expect("model path from gguf.toml");
	assert!(
		gguf.exists(),
		"perf model {model_key} is not present on this machine ({})",
		gguf.display()
	);
	let prompt = long_prompt(&gguf).expect("build >=2k-token prompt");
	let mut session = ChatSession::open(&gguf, &mut |_toks: &[Tok]| true)
		.expect("session open")
		.session()
		.expect("session cancelled");
	let mut generated = 0usize;
	let mut first: Option<f64> = None;
	let wall = Instant::now();
	let summary = session
		.generate_in(&prompt, &mut |_toks: &[Tok]| {
			let _stamp = first.get_or_insert_with(|| return wall.elapsed().as_secs_f64());
			generated += 1;
			return generated < gen_tokens;
		})
		.expect("generate_in returned Err");
	let elapsed = wall.elapsed().as_secs_f64();
	let last = summary.lines().last().unwrap_or_default();
	eprintln!("{tag}: TTFT {:.2}s, {last}", first.unwrap_or(f64::NAN));
	eprintln!(
		"{tag}: {generated} tokens in {elapsed:.2}s wall, {:.2} tok/s (harness)",
		generated as f64 / elapsed.max(1e-9)
	);
	assert!(
		generated >= 1,
		"generation produced no tokens — decode path is broken"
	);
}

#[test]
fn resident_decode_perf_qwen3() {
	measure(MODEL_KEY, GEN_TOKENS, "kv_perf");
}

#[test]
fn resident_decode_perf_lfm2_scan() {
	measure(SCAN_MODEL_KEY, SCAN_GEN_TOKENS, "kv_perf_scan");
}
