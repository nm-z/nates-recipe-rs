
use anyhow::{Context, Result};
use recipe_infer::llm::{greedy, greedy_windowed, last_logits};
use std::fs;
use std::path::{Path, PathBuf};

fn fixtures_dir() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/datasets/llamacpp-archs-seed42")
}

fn prompt_tokens() -> Result<Vec<u32>> {
	let text = fs::read_to_string(fixtures_dir().join("tokens.txt")).context("tokens.txt")?;
	let mut toks = Vec::new();
	for line in text.lines().filter(|l| !l.trim().is_empty()) {
		toks.push(line.trim().parse::<u32>().with_context(|| format!("token {line:?}"))?);
	}
	anyhow::ensure!(!toks.is_empty(), "tokens.txt empty");
	toks.truncate(6);
	return Ok(toks);
}

fn full_recompute_greedy(gguf: &Path, toks: &[u32], n: usize) -> Result<Vec<u32>> {
	let mut seq = toks.to_vec();
	let mut generated = Vec::with_capacity(n);
	for _ in 0..n {
		let logits = last_logits(gguf, &seq)?;
		let mut best = 0usize;
		let mut bv = f64::MIN;
		for (i, &v) in logits.iter().enumerate() {
			if v > bv {
				bv = v;
				best = i;
			}
		}
		seq.push(best as u32);
		generated.push(best as u32);
	}
	return Ok(generated);
}

fn assert_cached_matches_full(fixture: &str) {
	let gguf = fixtures_dir().join(fixture);
	if !gguf.exists() {
		panic!("fixture {fixture} missing at {}", gguf.display());
	}
	let toks = prompt_tokens().expect("prompt tokens");
	let n = 6;
	let cached = greedy(&gguf, &toks, n).expect("cached incremental greedy");
	let full = full_recompute_greedy(&gguf, &toks, n).expect("full-recompute greedy");
	assert_eq!(
		cached, full,
		"{fixture}: cached incremental decode diverges from full recompute\n  cached: {cached:?}\n  full:   {full:?}"
	);
}

#[test]
fn cached_matches_full_llama_gqa() {
	assert_cached_matches_full("llama-dense.gguf");
}

#[test]
fn cached_matches_full_qwen3_qknorm() {
	assert_cached_matches_full("qwen3-dense.gguf");
}

#[test]
fn cached_matches_full_deepseek2_mla() {
	assert_cached_matches_full("deepseek2-moe.gguf");
}

#[test]
fn cached_matches_full_mamba1() {
	assert_cached_matches_full("mamba-dense.gguf");
}

#[test]
fn cached_matches_full_mamba2() {
	assert_cached_matches_full("mamba2-dense.gguf");
}

#[test]
fn cached_matches_full_jamba_hybrid() {
	assert_cached_matches_full("jamba-dense.gguf");
}

#[test]
fn cached_matches_full_qwen3next_deltanet() {
	assert_cached_matches_full("qwen3next-moe.gguf");
}

#[test]
fn cached_matches_full_kimi_kda() {
	assert_cached_matches_full("kimi-linear-moe.gguf");
}

#[test]
fn cached_matches_full_lfm2_shortconv() {
	assert_cached_matches_full("lfm2-dense.gguf");
}

#[test]
fn cached_matches_full_plamo2_hybrid() {
	assert_cached_matches_full("plamo2-dense.gguf");
}

#[test]
fn cached_matches_full_minicpm3_naive_mla() {
	assert_cached_matches_full("minicpm3-dense.gguf");
}

#[test]
fn cached_matches_full_talkie() {
	assert_cached_matches_full("talkie-dense.gguf");
}

#[test]
fn cached_matches_full_for_falcon_h1() {
	assert_cached_matches_full("falcon-h1-dense.gguf");
}

#[test]
fn cached_matches_full_granitehybrid_mixerffn() {
	assert_cached_matches_full("granitehybrid-dense.gguf");
}

#[test]
fn cached_matches_full_nemotron_h_triage() {
	assert_cached_matches_full("nemotron_h-dense.gguf");
}

fn assert_spill_matches_resident(fixture: &str) {
	let gguf = fixtures_dir().join(fixture);
	if !gguf.exists() {
		panic!("fixture {fixture} missing at {}", gguf.display());
	}
	let toks = prompt_tokens().expect("prompt tokens");
	let n = 6;
	let resident = greedy(&gguf, &toks, n).expect("resident greedy");
	let prompt_cross =
		greedy_windowed(&gguf, &toks, n, toks.len() - 2).expect("prompt-crossing greedy");
	assert_eq!(
		prompt_cross, resident,
		"{fixture}: prompt longer than the VRAM window diverges from resident decode\n  spill:    {prompt_cross:?}\n  resident: {resident:?}"
	);
	let gen_cross =
		greedy_windowed(&gguf, &toks, n, toks.len() + 2).expect("generation-crossing greedy");
	assert_eq!(
		gen_cross, resident,
		"{fixture}: generation crossing the VRAM window diverges from resident decode\n  spill:    {gen_cross:?}\n  resident: {resident:?}"
	);
}

#[test]
fn spill_matches_resident_llama_gqa() {
	assert_spill_matches_resident("llama-dense.gguf");
}

#[test]
fn spill_matches_resident_qwen3_qknorm() {
	assert_spill_matches_resident("qwen3-dense.gguf");
}

#[test]
fn spill_matches_resident_deepseek2_mla() {
	assert_spill_matches_resident("deepseek2-moe.gguf");
}

#[test]
fn spill_matches_resident_jamba_hybrid() {
	assert_spill_matches_resident("jamba-dense.gguf");
}

#[test]
fn spill_matches_resident_qwen3next_deltanet() {
	assert_spill_matches_resident("qwen3next-moe.gguf");
}

#[test]
fn spill_matches_resident_kimi_kda_mla() {
	assert_spill_matches_resident("kimi-linear-moe.gguf");
}

#[test]
fn spill_matches_resident_plamo2_hybrid() {
	assert_spill_matches_resident("plamo2-dense.gguf");
}

#[test]
fn spill_matches_resident_minicpm3_asym_widths() {
	assert_spill_matches_resident("minicpm3-dense.gguf");
}

#[test]
fn spill_matches_resident_talkie() {
	assert_spill_matches_resident("talkie-dense.gguf");
}

#[test]
fn spill_matches_resident_for_falcon_h1() {
	assert_spill_matches_resident("falcon-h1-dense.gguf");
}

#[test]
fn spill_matches_resident_granitehybrid() {
	assert_spill_matches_resident("granitehybrid-dense.gguf");
}

#[test]
fn spill_matches_resident_nemotron_h() {
	assert_spill_matches_resident("nemotron_h-dense.gguf");
}

#[test]
fn one_decode_method_no_second_path() {
	let root = Path::new(env!("CARGO_MANIFEST_DIR"));
	let llm = fs::read_to_string(root.join("recipe-infer/src/llm.rs")).expect("llm.rs");
	assert!(!llm.contains("fn decode_scan"), "decode_scan fallback still present");
	assert!(!llm.contains("has_scan_op"), "has_scan_op decode fork still present");
	assert_eq!(
		llm.matches("fn decode_cached").count(),
		1,
		"expected exactly one cached decode function"
	);
	let common = fs::read_to_string(root.join("recipe-infer/src/models/common.rs")).expect("common.rs");
	assert!(!common.contains("gpu_gqa_attn"), "obsolete gpu_gqa_attn symbol present");
	assert!(!common.contains("gpu_mla_attn"), "obsolete gpu_mla_attn symbol present");
}
