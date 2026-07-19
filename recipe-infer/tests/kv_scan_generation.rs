
use recipe_infer::llm::greedy;
use std::path::{Path, PathBuf};

fn arch_fixture(name: &str) -> PathBuf {
	return Path::new(env!("CARGO_MANIFEST_DIR"))
		.join("../datasets/llamacpp-archs-seed42")
		.join(name);
}

fn assert_generates(name: &str) {
	let gguf = arch_fixture(name);
	let prompt = [1u32, 7, 13, 19, 25];
	let n_new = 8usize;
	let out = greedy(&gguf, &prompt, n_new).expect("scan/hybrid arch fell off the unified decode path");
	assert_eq!(
		out.len(),
		n_new,
		"{name}: unified decode produced {} of {n_new} requested tokens",
		out.len()
	);
	assert!(
		out.iter().all(|&t| (t as usize) < 128),
		"{name}: generated an out-of-vocab id (vocab 128): {out:?}"
	);
	eprintln!("{name}: generated {n_new} tokens through the one path: {out:?}");
}

#[test]
fn scan_arch_generates_through_the_one_path() {
	assert_generates("mamba2-dense.gguf");
}

#[test]
fn hybrid_arch_generates_through_the_one_path() {
	assert_generates("falcon-h1-dense.gguf");
}
