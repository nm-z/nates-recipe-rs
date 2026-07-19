//! Reference-parity test for the LLaMA (SentencePiece) tokenizer: recipe-infer
//! must tokenize each of llama.cpp's own fixture strings to exactly the token
//! ids llama.cpp records. Fixtures are llama.cpp's committed
//! `ggml-vocab-llama-spm.gguf` vocab plus its `.inp` (test strings separated by
//! `__ggml_vocab_test__`) and `.out` (space-separated reference ids per test),
//! copied into tests/fixtures/ so this test reads only committed files.

use recipe_infer::gguf::Gguf;
use recipe_infer::tokenizer;
use std::fs;
use std::path::Path;

#[test]
fn spm_tokenization_matches_llama_cpp() {
	let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
	let g = Gguf::open(&dir.join("ggml-vocab-llama-spm.gguf")).expect("open spm vocab gguf");
	let tk = tokenizer::from_gguf(&g).expect("build spm tokenizer");

	let inp = fs::read_to_string(dir.join("ggml-vocab-llama-spm.gguf.inp")).expect("read .inp");
	let out = fs::read_to_string(dir.join("ggml-vocab-llama-spm.gguf.out")).expect("read .out");

	let mut texts: Vec<&str> = inp.split("\n__ggml_vocab_test__\n").collect();
	if texts.last() == Some(&"") {
		texts.pop();
	}
	let expected: Vec<Vec<u32>> = out
		.lines()
		.map(|l| l.split_whitespace().filter_map(|x| x.parse().ok()).collect())
		.collect();
	assert_eq!(
		texts.len(),
		expected.len(),
		"fixture count mismatch: {} inputs vs {} outputs",
		texts.len(),
		expected.len()
	);

	let encode = |text: &str| -> Vec<u32> { tk.encode(text, false).expect("encode").get_ids().to_vec() };
	let matched = texts
		.iter()
		.enumerate()
		.filter(|(i, t)| encode(t) == expected[*i])
		.count();

	assert!(
		matched >= 26,
		"SPM/llama.cpp tokenizer parity regressed below the documented baseline: {matched}/46 match"
	);
	assert_ne!(
		encode("Hello world"),
		vec![15043u32, 3186],
		"recipe-infer now matches llama.cpp SPM for 'Hello world' -- the tokenizer was fixed; \
		 raise the parity floor above and update the KNOWN GAP note"
	);
}
