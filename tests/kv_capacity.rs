use gpu_core::memory::device_alloc_count;
use recipe_infer::llm::{ChatSession, Tok};
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
		.expect("generate_in kept refusing instead of decoding");
	return n;
}

#[test]
fn cache_growth_keeps_decoding_without_fresh_device_memory() {
	let gguf = stories_fixture();
	let mut session = ChatSession::open(&gguf, &mut |_toks: &[Tok]| true)
		.expect("session open")
		.session()
		.expect("session cancelled")
		.temp(0.0);
	let sentence = "Once upon a time there was a little girl who lived in a big house. ";
	let mut prompt = sentence.repeat(250);
	let budget = 2usize;
	let n1 = run_turn(&mut session, &prompt, budget);
	assert!(
		n1 >= 1,
		"cold multi-thousand-row prefill produced no tokens"
	);
	let allocs_after_first = device_alloc_count();
	let mut produced = n1;
	for turn in 1..10 {
		prompt.push_str(sentence);
		let n = run_turn(&mut session, &prompt, budget);
		assert!(
			n >= 1,
			"turn {turn} produced no tokens (growth stalled the decode)"
		);
		produced += n;
	}
	let alloc_delta = device_alloc_count() - allocs_after_first;
	eprintln!("10 growing turns, {produced} tokens generated, device alloc delta after first turn = {alloc_delta}");
	assert_eq!(
		alloc_delta, 0,
		"cache/arena growth performed {alloc_delta} fresh device allocations mid-run — growth must ride the claim and the spill tiers, never new allocs"
	);
}
