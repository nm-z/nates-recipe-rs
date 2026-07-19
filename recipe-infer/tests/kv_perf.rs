
use std::time::Instant;
use std::path::PathBuf;
use anyhow::{Context, Result, bail};
use recipe_infer::gguf::Gguf;
use recipe_infer::llm::{ChatSession, Tok};
use recipe_infer::tokenizer::from_gguf;

const MODEL_KEY: &str = "qwen3-0.6b-q8_0";
const MIN_PROMPT_TOKENS: usize = 2000;
const GEN_TOKENS: usize = 3;

const SCAN_MODEL_KEY: &str = "lfm2.5-1.2b";
const SCAN_GEN_TOKENS: usize = 3;

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

fn measure(model_key: &str, gen_tokens: usize, tag: &str) {
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
		.expect("session cancelled")
		.temp(0.0);
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
