//! Verifies the architecture dispatch contract: only architectures whose graph
//! is verified against llama.cpp and whose tensors resolve are reported
//! supported, using the real GGUF `general.architecture` strings. Everything
//! else is rejected (there is no silent fallback in `models::dispatch`).

use recipe_infer::llm::{arch_supported, supported_archs};

#[test]
fn only_verified_archs_are_supported() {
	for arch in ["llama", "xverse", "qwen3"] {
		assert!(arch_supported(arch), "{arch} is verified and must be supported");
	}
	assert_eq!(supported_archs(), &["llama", "xverse", "qwen3"]);
}

#[test]
fn unverified_archs_are_rejected() {
	// Real GGUF arch strings that are NOT yet verified end-to-end. Reporting any
	// of these as supported (or silently decoding them) is the bug this guards.
	for arch in [
		"gpt2", "bert", "qwen2", "qwen2moe", "gemma2", "mamba", "rwkv6", "t5",
		"deepseek2", "falcon", "phi3", "starcoder2",
	] {
		assert!(
			!arch_supported(arch),
			"{arch} must not be reported supported until its graph and logits are verified"
		);
	}
}
