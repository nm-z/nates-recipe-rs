use crate::gguf::Gguf;
use anyhow::{Result, anyhow};
pub use tokenizers::Tokenizer;

pub fn from_file(path: &std::path::Path) -> Result<Tokenizer> {
	Tokenizer::from_file(path).map_err(|e| anyhow!("tokenizer {}: {e}", path.display()))
}

pub fn from_gguf(g: &Gguf) -> Result<Tokenizer> {
	let pieces = g.str_arr("tokenizer.ggml.tokens")?;
	let scores = g.f32_arr("tokenizer.ggml.scores")?;
	let unk = g.u32_kv("tokenizer.ggml.unknown_token_id").unwrap_or(3) as usize;
	let pairs: Vec<(String, f64)> = pieces
		.into_iter()
		.zip(scores.iter().map(|&s| s as f64))
		.collect();
	let uni = tokenizers::models::unigram::Unigram::from(pairs, Some(unk), true)
		.map_err(|e| anyhow!("unigram: {e}"))?;
	let mut tk = Tokenizer::new(uni);
	tk.with_normalizer(Some(tokenizers::normalizers::Sequence::new(vec![
		tokenizers::normalizers::Prepend::new("\u{2581}".to_string()).into(),
		tokenizers::normalizers::Replace::new(" ", "\u{2581}")
			.map_err(|e| anyhow!("replace: {e}"))?
			.into(),
	])));
	Ok(tk)
}

pub fn gguf_vocab(g: &Gguf, size: usize) -> Result<Vec<String>> {
	let mut v = g.str_arr("tokenizer.ggml.tokens")?;
	v.resize(size, String::new());
	Ok(v)
}
