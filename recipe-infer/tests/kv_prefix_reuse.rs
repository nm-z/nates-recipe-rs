//! Prefix-reuse proof: turn 2 of a resident [`ChatSession`] with a shared
//! prefix must append only the suffix K/V, never re-prefill the whole sequence.
//! The cache length is not visible from outside the crate, so the assertion
//! rides the gpu-core memory ledger (the single choke point every HIP transfer
//! goes through, per the CLAUDE.md ledger law): each forward uploads its new
//! embedding rows H2D, so H2D BYTES scale with the rows actually forwarded.
//!
//! Generation is capped at ONE accepted token so each turn is exactly one
//! forward call — this isolates the prefill from the per-step lm-head weight
//! streaming, which is a fixed per-call H2D cost that would otherwise swamp the
//! signal. Turn 1 (cold) forwards the whole N-row prompt; turn 2, with the same
//! prompt fully cached, forwards a single suffix row. With a long prompt the
//! N-row base upload dominates the shared fixed cost, so turn 2's H2D collapses
//! to a small fraction of turn 1's iff the prefix K/V was reused. A re-prefill
//! would forward all N rows again and make the two turns comparable.

use gpu_core::memory::xfer_bytes;
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
		.expect("generate_in");
	return n;
}

#[test]
fn second_turn_uploads_scale_with_suffix_not_total_length() {
	let gguf = stories_fixture();
	let mut session = ChatSession::open(&gguf, &mut |_toks: &[Tok]| true)
		.expect("session open")
		.session()
		.expect("session cancelled");
	let prompt = "Once upon a time there was a little girl. ".repeat(60);
	let budget = 1usize;
	let h2d_before = xfer_bytes().h2d;
	let n1 = run_turn(&mut session, &prompt, budget);
	let turn1 = xfer_bytes().h2d - h2d_before;
	assert!(n1 >= 1, "turn 1 produced no tokens");
	let h2d_mid = xfer_bytes().h2d;
	let n2 = run_turn(&mut session, &prompt, budget);
	let turn2 = xfer_bytes().h2d - h2d_mid;
	assert!(n2 >= 1, "turn 2 produced no tokens");
	let h2d_mid2 = xfer_bytes().h2d;
	let n3 = run_turn(&mut session, &prompt, budget);
	let turn3 = xfer_bytes().h2d - h2d_mid2;
	assert!(n3 >= 1, "turn 3 produced no tokens");
	eprintln!(
		"H2D bytes: turn1 (cold prefill of the full ~660-row prompt) = {turn1}, turn2 (same prompt fully cached, 1 suffix row) = {turn2}, turn3 (same again, longer cached total) = {turn3}"
	);
	assert!(
		turn1 > 0,
		"turn 1 moved no H2D bytes — the ledger is not seeing the forward"
	);
	assert!(
		turn2 < turn1,
		"turn 2 with a fully shared prefix moved {turn2} H2D bytes, not less than the {turn1}-byte cold prefill: prefix K/V not reused"
	);
	assert_eq!(
		turn3, turn2,
		"two 1-suffix-row turns at different cached totals moved different H2D byte counts — turn work scales with the total, not the suffix"
	);
}
