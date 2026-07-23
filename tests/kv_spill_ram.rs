use gpu_core::memory::{device_alloc_count, xfer_calls};
use recipe_infer::llm::{ChatSession, Tok, greedy, last_logits};
use std::cmp::Ordering;
use std::path::{Path, PathBuf};

fn stories_fixture() -> PathBuf {
	return Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/stories-f32.gguf");
}

fn run_turn(session: &mut ChatSession, prompt: &str, budget: usize) -> usize {
	let mut n = 0usize;
	session
		.generate_in(prompt, &mut |_toks: &[Tok]| {
			n += 1;
			return n < budget;
		})
		.expect("generate_in stopped decoding under cache growth");
	return n;
}

fn argmax(logits: &[f64]) -> u32 {
	let mut best = 0usize;
	let mut bv = f64::NEG_INFINITY;
	for (i, &v) in logits.iter().enumerate() {
		if v.partial_cmp(&bv) == Some(Ordering::Greater) {
			bv = v;
			best = i;
		}
	}
	return best as u32;
}

#[test]
fn spill_growth_moves_bytes_without_new_device_memory() {
	let gguf = stories_fixture();
	let mut session = ChatSession::open(&gguf, &mut |_toks: &[Tok]| true)
		.expect("session open")
		.session()
		.expect("session cancelled")
		.temp(0.0);
	let sentence = "Once upon a time there was a little girl who lived in a big house. ";
	let mut prompt = sentence.repeat(60);
	run_turn(&mut session, &prompt, 2);
	let base_alloc = device_alloc_count();
	let base_x = xfer_calls();
	let mut produced = 0usize;
	for turn in 1..6 {
		prompt.push_str(&sentence.repeat(2));
		let n = run_turn(&mut session, &prompt, 2);
		assert!(
			n >= 1,
			"turn {turn} produced no tokens — growth stalled the decode"
		);
		produced += n;
	}
	let alloc_delta = device_alloc_count() - base_alloc;
	let x = xfer_calls();
	eprintln!(
		"5 growing turns, {produced} tokens; device alloc delta = {alloc_delta}; transfer calls d2h {} -> {}, h2d {} -> {}",
		base_x.d2h, x.d2h, base_x.h2d, x.h2d
	);
	assert_eq!(
		alloc_delta, 0,
		"cache growth performed {alloc_delta} fresh device allocations — growth must ride the claim and spill tiers, never new device memory"
	);
	assert!(
		x.d2h > base_x.d2h && x.h2d > base_x.h2d,
		"the ledger saw no transfer traffic across 5 growing turns (d2h {} -> {}, h2d {} -> {}) — cache movement must show as transfers through the choke points",
		base_x.d2h,
		x.d2h,
		base_x.h2d,
		x.h2d
	);
}

#[test]
fn resident_cache_content_matches_cold_recompute() {
	let gguf = stories_fixture();
	let prompt: &[u32] = &[1, 403, 407, 261, 378];
	let cached = greedy(&gguf, prompt, 1).expect("cached first token");
	let cold = argmax(&last_logits(&gguf, prompt).expect("cold recompute logits"));
	assert_eq!(
		cached[0], cold,
		"resident cache's first token {} disagrees with a cold full recompute {} — a row's logits must not depend on its storage tier",
		cached[0], cold
	);
	eprintln!("resident cached token == cold recompute token ({})", cold);
}
