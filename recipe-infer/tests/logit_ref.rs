
use recipe_infer::llm::greedy;
use std::path::Path;

const PROMPT_TOKS: &[u32] = &[1, 403, 407, 261, 378];

const LLAMA_GREEDY: &[u32] = &[432, 383, 286, 261, 376, 298, 315, 421];

const STABLE: usize = 6;

#[test]
fn stories_greedy_matches_llama_cpp() {
	let model = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/stories-f32.gguf");
	let generated = greedy(&model, PROMPT_TOKS, LLAMA_GREEDY.len()).expect("recipe-infer greedy");
	assert_eq!(
		&generated[..STABLE],
		&LLAMA_GREEDY[..STABLE],
		"recipe-infer greedy decode diverges from llama.cpp within the stable prefix\n  \
		 recipe-infer: {generated:?}\n  llama.cpp:    {LLAMA_GREEDY:?}"
	);
}
