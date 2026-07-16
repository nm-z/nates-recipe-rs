//! Apples-to-apples tokenizer parity against llama.cpp, one row per vocab.
//!
//! Fixtures in `datasets/llamacpp-vocabs-tokenizer/` are llama.cpp's own
//! committed tokenizer test set (`models/ggml-vocab-*.gguf` plus `.inp` test
//! strings separated by `__ggml_vocab_test__` and `.out` reference ids per
//! case), copied verbatim. recipe-infer must tokenize every case of every
//! vocab to exactly llama.cpp's ids.
//!
//! Verdict per vocab on stderr:
//!   OK     every case matches
//!   FAIL   tokenizer built, some cases mismatch (matched/total shown)
//!   ERROR  vocab gguf did not open or no tokenizer could be built from it
//!   NOREF  gguf present without .inp/.out reference pair
//!
//! Passes only at full parity; red rows are the burn-down baseline.

use anyhow::{Context, Result};
use recipe_infer::gguf::Gguf;
use recipe_infer::tokenizer;
use std::fs;
use std::path::{Path, PathBuf};

fn cases(dir: &Path, stem: &str) -> Result<(Vec<String>, Vec<Vec<u32>>)> {
	let inp = fs::read_to_string(dir.join(format!("{stem}.gguf.inp"))).context(".inp")?;
	let out = fs::read_to_string(dir.join(format!("{stem}.gguf.out"))).context(".out")?;
	let mut texts: Vec<String> =
		inp.split("\n__ggml_vocab_test__\n").map(str::to_string).collect();
	// The .inp ends with a trailing separator, so split yields a final empty
	// element that is not a test case; drop it (interior empty tests are kept).
	if texts.last().is_some_and(String::is_empty) {
		texts.pop();
	}
	let expected: Vec<Vec<u32>> = out
		.lines()
		.map(|l| l.split_whitespace().filter_map(|x| x.parse().ok()).collect())
		.collect();
	anyhow::ensure!(
		texts.len() == expected.len(),
		"fixture count mismatch: {} inputs vs {} outputs",
		texts.len(),
		expected.len()
	);
	return Ok((texts, expected));
}

fn matched_cases(dir: &Path, stem: &str) -> Result<(usize, usize)> {
	let g = Gguf::open(&dir.join(format!("{stem}.gguf")))?;
	let tk = tokenizer::from_gguf(&g)?;
	let (texts, expected) = cases(dir, stem)?;
	let mut matched = 0;
	for (text, want) in texts.iter().zip(expected.iter()) {
		let got = tk.encode(text.as_str(), false).map(|e| e.get_ids().to_vec());
		if got.is_ok_and(|ids| &ids == want) {
			matched += 1;
		}
	}
	return Ok((matched, texts.len()));
}

#[test]
fn vocabs_parity_vs_llama_cpp() {
	let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../datasets/llamacpp-vocabs-tokenizer");
	let mut vocabs: Vec<PathBuf> = fs::read_dir(&dir)
		.expect("fixtures dir")
		.filter_map(|e| e.ok().map(|e| e.path()))
		.filter(|p| p.extension().is_some_and(|x| x == "gguf"))
		.collect();
	vocabs.sort();
	assert!(!vocabs.is_empty(), "no vocab fixtures in {}", dir.display());

	let (mut ok, mut fail, mut err, mut noref) = (0u32, 0u32, 0u32, 0u32);
	eprintln!("|{:>22}|{:>26}|", "Vocab", "recipe-infer vs llama.cpp");
	eprintln!("|{}|{}|", "-".repeat(22), "-".repeat(26));
	for gguf in &vocabs {
		let stem = gguf.file_stem().and_then(|s| s.to_str()).unwrap_or("?");
		let name = stem.strip_prefix("ggml-vocab-").unwrap_or(stem);
		let has_ref = dir.join(format!("{stem}.gguf.inp")).exists()
			&& dir.join(format!("{stem}.gguf.out")).exists();
		let verdict = if !has_ref {
			noref += 1;
			"\x1b[1;33mNOREF\x1b[0m".to_string()
		} else {
			match matched_cases(&dir, stem) {
				Ok((m, n)) if m == n => {
					ok += 1;
					format!("\x1b[1;32mOK\x1b[0m ({m}/{n})")
				}
				Ok((m, n)) => {
					fail += 1;
					format!("\x1b[1;31mFAIL\x1b[0m ({m}/{n})")
				}
				Err(e) => {
					err += 1;
					format!("\x1b[1;31mERROR\x1b[0m {e:#}")
				}
			}
		};
		eprintln!("|{name:>22}|{verdict}|");
	}
	let referenced = ok + fail + err;
	eprintln!("tokenizer parity: {ok} OK, {fail} FAIL, {err} ERROR of {referenced} referenced ({noref} NOREF)");
	assert_eq!(
		(fail, err),
		(0, 0),
		"full tokenizer parity not reached: {ok}/{referenced} vocabs match llama.cpp"
	);
}
