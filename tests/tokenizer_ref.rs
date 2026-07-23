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
		.map(|l| {
			l.split_whitespace()
				.filter_map(|x| x.parse().ok())
				.collect()
		})
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

	eprintln!("spm parity: {matched}/{}", texts.len());
	assert!(
		matched >= 31,
		"SPM/llama.cpp tokenizer parity regressed below the documented baseline: {matched}/46 match"
	);
	assert_eq!(
		encode("Hello world"),
		vec![15043u32, 3186],
		"'Hello world' must encode to llama.cpp's SPM ids; this probe was fixed and is locked"
	);
}
