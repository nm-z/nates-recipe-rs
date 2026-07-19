//! Turn-2 work-scaling gate: a resident 2-turn cached chat must do turn-2 work
//! proportional to the NEW tokens (the suffix), not the whole sequence — the
//! cached prefix's K/V is reused, so turn 2 forwards only the suffix plus its
//! generation. The proof rides the gpu-core memory ledger (the single choke
//! point every HIP transfer passes through, per the CLAUDE.md ledger law).
//!
//! Which ledger counter proves it, and which does not:
//!   * H2D BYTES (used here): every forward uploads its new embedding rows H2D,
//!     so H2D bytes scale with the rows actually forwarded. Turn 1 uploads the
//!     whole N-row prompt; turn 2, with that prompt cached, uploads a one-row
//!     suffix plus generation. Turn-2 H2D bytes collapse to a small fraction of
//!     turn 1's iff the prefix was reused; a re-prefill would re-upload all N
//!     rows and make the turns comparable.
//!   * device_alloc_count DELTA (used here): must stay 0 across both turns — the
//!     one-claim law; suffix work rides the resident claim, never a new alloc.
//!   * H2D CALL count / kernel LAUNCH count (NOT asserted): these do not cleanly
//!     separate the turns. A cold prefill of N rows is ONE batched forward (one
//!     set of per-layer kernel launches), while the cross-turn suffix is stepped
//!     one token at a time (one forward per suffix token). So launch/call COUNT
//!     tracks the number of forward passes, not the rows per pass — the work per
//!     dispatch (GEMM m-dimension) is what scales with the suffix. That scaling
//!     is a kernel-grid fact, proven at the kernel level with rocprofv3 below.
//!
//! Kernel-level version (run by a human; not a suite assertion):
//!
//!   # Build the harness binary, then trace ONE run of the two-turn chat:
//!   timeout 8 rocprofv3 --hip-trace --kernel-trace -d /tmp/kvdisp -- \
//!     ./target/release/deps/kv_dispatch_count-<hash> \
//!     --exact turn2_h2d_scales_with_suffix_not_total --nocapture --test-threads=1
//!
//!   # rocprofv3 hangs at teardown; the SQLite DB survives the kill. Query the
//!   # attention/GEMM dispatch grid sizes and count per turn window (turn
//!   # boundaries are the two "prompt tokens=..." lines on stderr). Grid_x of the
//!   # flash-attention dispatch encodes t_q*nqh, so t_q (the rows forwarded) is
//!   # read straight off the grid:
//!   DB=$(find /tmp/kvdisp -name '*_results.db')
//!   sqlite3 "$DB" "SELECT k.string AS kernel, d.grid_x, COUNT(*) \
//!     FROM rocpd_kernel_dispatch d \
//!     JOIN rocpd_info_kernel_symbol k ON d.kernel_id=k.id \
//!     WHERE k.string LIKE '%flash_gqa%' OR k.string LIKE '%flash_mla%' \
//!     GROUP BY kernel, d.grid_x ORDER BY d.grid_x DESC;"
//!
//!   Expected: turn 1 shows a large grid_x (N-row prefill, grid_x = N*nqh); turn
//!   2 shows only grid_x = 1*nqh (single-row suffix + single-row decode steps) —
//!   NO N-row dispatch. A re-prefill regression re-introduces the large grid_x in
//!   turn 2, which is the failure this gate catches.
//!
//! Fixture: the committed stories-f32 (llama, working tokenizer). Serial, one GPU
//! process per test under the orchestrator.

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
		.expect("session cancelled");
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
