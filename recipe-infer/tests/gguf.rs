use recipe_infer::dequant::{block_layout, nbytes_for};
use recipe_infer::gguf::Gguf;
use std::path::Path;

#[test]
fn open_gemma_vocab_and_validate_layout() {
	let path = Path::new(env!("CARGO_MANIFEST_DIR"))
		.join("..")
		.join("datasets")
		.join("gguf-test")
		.join("ggml-vocab-gemma-4.gguf");
	let g = Gguf::open(&path).expect("open committed gemma vocab gguf");
	assert!(
		g.data_start >= 4,
		"data_start past the GGUF magic: {}",
		g.data_start
	);
	let toks = g
		.str_arr("tokenizer.ggml.tokens")
		.expect("tokenizer.ggml.tokens string array");
	assert!(!toks.is_empty(), "tokenizer has at least one token");
	for (name, info) in &g.tensors {
		let (block_bytes, block_elems) =
			block_layout(info.ggml_type).expect("block_layout for tensor type");
		let elems: usize = info.dims.iter().product();
		assert_eq!(
			elems % block_elems,
			0,
			"tensor {name}: {elems} elems is not a multiple of block {block_elems}"
		);
		assert_eq!(
			info.nbytes,
			(elems / block_elems) * block_bytes,
			"tensor {name}: nbytes inconsistent with block_layout"
		);
		assert_eq!(
			info.nbytes,
			nbytes_for(info.ggml_type, elems).expect("nbytes_for tensor type"),
			"tensor {name}: nbytes_for disagrees"
		);
		assert!(
			info.offset >= g.data_start,
			"tensor {name}: offset {} before data_start {}",
			info.offset,
			g.data_start
		);
	}
}
