//! Proves the RAM headroom probe child entry: an unset env means "not a probe
//! child" (`None`), and a set budget is committed and touched, so surviving the
//! commit reports `Some(0)`. This is the CPU-only child half of the VRAM-shaped
//! host-RAM probe; the parent loop is never spawned from a test (its
//! `current_exe` would re-enter the suite binary).
use gpu_core::memory::ram_probe_ask;
use std::env;

#[test]
fn ram_probe_child_commits_and_reports() {
	assert_eq!(ram_probe_ask(), None, "unset RAM_PROBE must yield None");
	// SAFETY: single-threaded within this test; no other test reads RAM_PROBE.
	unsafe {
		env::set_var("RAM_PROBE", (64usize << 20i32).to_string());
	}
	let survived = ram_probe_ask();
	// SAFETY: single-threaded within this test; restore the clean process env.
	unsafe {
		env::remove_var("RAM_PROBE");
	}
	assert_eq!(
		survived,
		Some(0),
		"a 64 MiB host commit must survive and report Some(0)"
	);
}
