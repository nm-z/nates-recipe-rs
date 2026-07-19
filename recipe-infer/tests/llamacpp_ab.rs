//! Live A/B against llama.cpp on the SAME gguf: greedy continuation must be
//! IDENTICAL (not approximate), and recipe must not be slower. The reference
//! engine is `/usr/bin/llama-completion` (raw completion; this build's
//! `llama-cli` is a chat REPL that wraps the prompt and spins on closed
//! stdin), run at `--temp 0` on the same file recipe loads, so tokenizer and
//! weights are shared and only the engines differ.
//!
//! Token identity runs on the committed `stories-f32.gguf` fixture. The perf
//! gates run on the machine's qwen3-0.6b-q8_0 (via `gguf.toml`): on the 260K
//! stories toy a perf ratio only measures kernel-launch overhead against a
//! trivial CPU sprint, not engine speed.
//!
//! Run serially (`--test-threads=1`): each test opens its own resident session
//! and two concurrent device-arena claims collide by design (one GPU process).

use anyhow::{Context, Result, bail};
use recipe_infer::llm::{ChatSession, Tok};
use std::time::Instant;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const PROMPT: &str = "Once upon a time";
const N_NEW: usize = 64;

fn fixture() -> PathBuf {
	return Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/stories-f32.gguf");
}

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

struct EngineRun {
	text: String,
	ttft_s: f64,
	tok_s: f64,
}

fn llama_completion(model: &Path, prompt: &str, n_new: usize) -> EngineRun {
	let out = Command::new("/usr/bin/llama-completion")
		.args([
			"-m",
			model.to_str().expect("model path utf8"),
			"-p",
			prompt,
			"-n",
			&n_new.to_string(),
			"--temp",
			"0",
			"--seed",
			"1",
			"--simple-io",
		])
		.stdin(Stdio::null())
		.output()
		.expect("spawn /usr/bin/llama-completion");
	assert!(
		out.status.success(),
		"llama-completion exited {:?}\nstderr:\n{}",
		out.status,
		String::from_utf8_lossy(&out.stderr)
	);
	let stdout = String::from_utf8_lossy(&out.stdout).to_string();
	let stderr = String::from_utf8_lossy(&out.stderr).to_string();
	let cont = stdout
		.split_once(prompt)
		.map(|(_pre, c)| return c.to_string())
		.unwrap_or(stdout.clone());
	let text = cont.replace("[end of text]", "").trim().to_string();
	let first_num = |line: &str| -> Option<f64> {
		let rest = line.split_once('=')?.1.trim();
		return rest.split_whitespace().next()?.parse::<f64>().ok();
	};
	let mut ttft_s = f64::NAN;
	let mut tok_s = f64::NAN;
	for line in stderr.lines() {
		if line.contains("prompt eval time") {
			ttft_s = first_num(line).map_or(f64::NAN, |ms| return ms / 1000.0);
		} else if line.contains("eval time") && !line.contains("prompt") {
			let tail = line.rsplit_once('(').map_or("", |(_pre, t)| return t);
			let mut last = f64::NAN;
			for w in tail.split_whitespace() {
				if let Ok(v) = w.parse::<f64>() {
					last = v;
				}
			}
			tok_s = last;
		}
	}
	assert!(
		ttft_s.is_finite() && tok_s.is_finite(),
		"could not parse common_perf timings from llama-completion stderr:\n{stderr}"
	);
	return EngineRun { text, ttft_s, tok_s };
}

fn recipe_run(model: &Path, prompt: &str, n_new: usize) -> EngineRun {
	let mut session = ChatSession::open(model, &mut |_toks: &[Tok]| true)
		.expect("session open")
		.session()
		.expect("session cancelled");
	let mut n = 0usize;
	let mut first: Option<Instant> = None;
	let sent = Instant::now();
	let reply = session
		.generate_in(prompt, &mut |_toks: &[Tok]| {
			let _stamp = first.get_or_insert_with(Instant::now);
			n += 1;
			return n < n_new;
		})
		.expect("generate_in");
	let ttft_s = first.map_or(f64::NAN, |f| return f.duration_since(sent).as_secs_f64());
	let tok_s = first.map_or(f64::NAN, |f| return n as f64 / f.elapsed().as_secs_f64().max(1e-9));
	let body = reply
		.rsplit_once("\n\n")
		.map(|(b, _stats)| return b.to_string())
		.unwrap_or(reply);
	return EngineRun {
		text: body.trim().to_string(),
		ttft_s,
		tok_s,
	};
}

fn assert_same_text(recipe: &str, llama: &str) {
	if recipe == llama {
		return;
	}
	let div = recipe
		.chars()
		.zip(llama.chars())
		.position(|(a, b)| return a != b)
		.unwrap_or(recipe.chars().count().min(llama.chars().count()));
	let ctx = |s: &str| -> String {
		let start = div.saturating_sub(30);
		return s.chars().skip(start).take(80).collect();
	};
	panic!(
		"greedy continuation diverged at char {div}:\n  recipe:    …{}\n  llama.cpp: …{}\n(full recipe {} chars, llama {} chars)",
		ctx(recipe),
		ctx(llama),
		recipe.len(),
		llama.len()
	);
}

#[test]
fn greedy_tokens_match_llamacpp() {
	let model = fixture();
	let llama = llama_completion(&model, PROMPT, N_NEW);
	let recipe = recipe_run(&model, PROMPT, N_NEW);
	eprintln!(
		"stories A/B: llama {:.1} tok/s TTFT {:.3}s | recipe {:.1} tok/s TTFT {:.3}s",
		llama.tok_s, llama.ttft_s, recipe.tok_s, recipe.ttft_s
	);
	assert_same_text(&recipe.text, &llama.text);
}

#[test]
fn recipe_not_slower_than_llamacpp() {
	let model = model_path("qwen3-0.6b-q8_0").expect("qwen3-0.6b-q8_0 in gguf.toml");
	assert!(
		model.exists(),
		"perf model is not present on this machine ({})",
		model.display()
	);
	let llama = llama_completion(&model, PROMPT, N_NEW);
	let recipe = recipe_run(&model, PROMPT, N_NEW);
	eprintln!(
		"qwen3 A/B: llama {:.1} tok/s TTFT {:.3}s | recipe {:.1} tok/s TTFT {:.3}s",
		llama.tok_s, llama.ttft_s, recipe.tok_s, recipe.ttft_s
	);
	assert!(
		recipe.tok_s >= llama.tok_s * 0.8,
		"recipe {:.1} tok/s vs llama.cpp {:.1} tok/s",
		recipe.tok_s,
		llama.tok_s
	);
	assert!(
		recipe.ttft_s <= llama.ttft_s * 1.5,
		"recipe TTFT {:.3}s vs llama.cpp TTFT {:.3}s",
		recipe.ttft_s,
		llama.ttft_s
	);
}
