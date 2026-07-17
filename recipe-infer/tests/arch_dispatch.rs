//! Verifies the architecture support contract with the verified/composable
//! split: `arch_supported` reports only architectures whose parity fixtures
//! ALL measure OK against llama.cpp (entry by measurement, enforced by
//! archs_parity's regression gate); `arch_composable` reports what
//! `models::dispatch` can attempt. Verified is a subset of composable, and
//! unknown strings are rejected by both (no silent fallback).

use recipe_infer::llm::{arch_composable, arch_supported, composable_archs, supported_archs};

#[test]
fn verified_is_a_measured_subset_of_composable() {
	assert!(!supported_archs().is_empty(), "VERIFIED went empty");
	for arch in supported_archs() {
		assert!(arch_supported(arch), "{arch} listed but not reported supported");
		assert!(
			arch_composable(arch),
			"{arch} verified but not composable: roster and table diverged"
		);
	}
	for arch in ["qwen3", "xverse", "gemma2", "qwen2", "phi3"] {
		assert!(arch_supported(arch), "{arch} is measured OK and must be supported");
	}
}

#[test]
fn composable_but_unverified_is_not_supported() {
	for arch in ["bloom", "rwkv6", "t5", "mpt", "falcon", "gpt2"] {
		assert!(
			arch_composable(arch),
			"{arch} is in the dispatch table and must be composable"
		);
		assert!(
			!arch_supported(arch),
			"{arch} has no measured parity and must not be reported supported"
		);
	}
	assert!(composable_archs().len() > supported_archs().len());
}

#[test]
fn unknown_archs_are_rejected() {
	for arch in ["gpt7", "clip", "(unknown)", ""] {
		assert!(!arch_composable(arch), "{arch:?} must not be composable");
		assert!(!arch_supported(arch), "{arch:?} must not be supported");
	}
}
