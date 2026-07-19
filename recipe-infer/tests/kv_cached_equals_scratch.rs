//! Cached-decode exactness at the token level: the incremental cached path must
//! generate token-for-token what a from-scratch full recompute produces.
//!
//! [`recipe_infer::llm::greedy`] IS the cached path — one batched prefill of the
//! prompt, then one row per step appending K/V (attention layers) and carrying
//! scan/conv state (recurrent layers) through the resident cache. The scratch
//! oracle re-forwards the ENTIRE sequence every step through
//! [`recipe_infer::llm::last_logits`] (fresh model open, no cache) and greedily
//! extends it. Both apply the same monotone logit-scale/softcap before argmax,
//! so identical logits give identical picks — any divergence is the cache
//! state-carry disagreeing with a clean recompute. Equality is required, not
//! approximate.
//!
//! Fed raw token ids (bypassing the tokenizer, exactly as archs_parity /
//! logit_ref do) so this validates the decode math, not tokenization — which
//! also lets it run on the committed archs-parity fixtures whose seeded minimal
//! tokenizers don't build. Covered: a dense arch (llama, committed stories-f32),
//! a pure-scan arch (mamba2) and an attention/SSM hybrid (falcon-h1), both from
//! the committed archs-parity set. Each opens/frees its own claim; runs are
//! serial, one GPU process at a time under the suite orchestrator.

use recipe_infer::llm::{greedy, last_logits};
use std::cmp::Ordering;
use std::path::{Path, PathBuf};

fn stories_fixture() -> PathBuf {
	return Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/stories-f32.gguf");
}

fn arch_fixture(name: &str) -> PathBuf {
	return Path::new(env!("CARGO_MANIFEST_DIR"))
		.join("../datasets/llamacpp-archs-seed42")
		.join(name);
}

fn argmax(logits: &[f64], vocab: usize) -> u32 {
	let mut best = 0usize;
	let mut bv = f64::NEG_INFINITY;
	for (i, &v) in logits.iter().take(vocab).enumerate() {
		if v.partial_cmp(&bv) == Some(Ordering::Greater) {
			bv = v;
			best = i;
		}
	}
	return best as u32;
}

fn scratch_generate(gguf: &Path, prompt: &[u32], n_new: usize) -> Vec<u32> {
	let vocab = last_logits(gguf, prompt)
		.expect("scratch prime logits")
		.len();
	let mut seq = prompt.to_vec();
	let mut out = Vec::with_capacity(n_new);
	for _ in 0..n_new {
		let logits = last_logits(gguf, &seq).expect("scratch step logits");
		let next = argmax(&logits, vocab);
		out.push(next);
		seq.push(next);
	}
	return out;
}

fn assert_cached_equals_scratch(gguf: &Path, prompt: &[u32], n_new: usize) {
	let cached = greedy(gguf, prompt, n_new).expect("cached greedy");
	let scratch = scratch_generate(gguf, prompt, n_new);
	assert_eq!(
		cached,
		scratch,
		"cached incremental decode diverges from the from-scratch full recompute for {}:\n  cached:  {cached:?}\n  scratch: {scratch:?}",
		gguf.display()
	);
	eprintln!(
		"{}: {n_new} tokens, cached == scratch ({cached:?})",
		gguf.display()
	);
}

#[test]
fn dense_cached_equals_scratch() {
	assert_cached_equals_scratch(&stories_fixture(), &[1, 403, 407, 261, 378], 6);
}

#[test]
fn scan_cached_equals_scratch() {
	assert_cached_equals_scratch(
		&arch_fixture("mamba2-dense.gguf"),
		&[1, 5, 9, 13, 17, 21],
		6,
	);
}

#[test]
fn hybrid_cached_equals_scratch() {
	assert_cached_equals_scratch(
		&arch_fixture("falcon-h1-dense.gguf"),
		&[1, 5, 9, 13, 17, 21],
		6,
	);
}
