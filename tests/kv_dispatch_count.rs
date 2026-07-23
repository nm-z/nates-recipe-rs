use gpu_core::memory::{device_alloc_count, xfer_bytes};
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
fn turn2_h2d_scales_with_suffix_not_total() {
	let gguf = stories_fixture();
	let mut session = ChatSession::open(&gguf, &mut |_toks: &[Tok]| true)
		.expect("session open")
		.session()
		.expect("session cancelled")
		.temp(0.0);
	let prompt = "Once upon a time there was a little girl. ".repeat(60);
	let budget = 1usize;

	let h2d0 = xfer_bytes().h2d;
	let alloc0 = device_alloc_count();
	let n1 = run_turn(&mut session, &prompt, budget);
	let turn1_h2d = xfer_bytes().h2d - h2d0;
	let alloc_after_t1 = device_alloc_count();

	let h2d1 = xfer_bytes().h2d;
	let n2 = run_turn(&mut session, &prompt, budget);
	let turn2_h2d = xfer_bytes().h2d - h2d1;
	let alloc_after_t2 = device_alloc_count();

	eprintln!(
		"turn1: {n1} tok, H2D {turn1_h2d} B (cold prefill of ~660 rows) | turn2: {n2} tok, H2D {turn2_h2d} B (prefix cached, 1-row suffix)"
	);
	eprintln!("device allocs: start {alloc0}, after turn1 {alloc_after_t1}, after turn2 {alloc_after_t2}");
	assert!(n1 >= 1 && n2 >= 1, "a turn produced no tokens");
	assert!(
		turn1_h2d > 0,
		"turn 1 moved no H2D bytes — ledger not seeing the forward"
	);
	assert_eq!(
		alloc_after_t2, alloc0,
		"device allocations grew across the two turns (start {alloc0}, end {alloc_after_t2}) — suffix work must ride the resident claim, never a new alloc"
	);
	let h2d2 = xfer_bytes().h2d;
	let n3 = run_turn(&mut session, &prompt, budget);
	let turn3_h2d = xfer_bytes().h2d - h2d2;
	assert!(n3 >= 1, "turn 3 produced no tokens");
	assert!(
		turn2_h2d < turn1_h2d,
		"turn-2 H2D {turn2_h2d} B is not less than the cold prefill's {turn1_h2d} B: prefix K/V not reused"
	);
	assert_eq!(
		turn3_h2d, turn2_h2d,
		"two 1-suffix-row turns at different cached totals moved different H2D byte counts — turn work scales with the total, not the suffix"
	);
}
