use crate::gguf::Gguf;
use anyhow::{Result, anyhow};
use std::path::Path;
use tokenizers::models::bpe::{BPE, Vocab};
use tokenizers::pre_tokenizers::byte_level::ByteLevel;
pub use tokenizers::Tokenizer;

pub fn from_file(path: &Path) -> Result<Tokenizer> {
	Tokenizer::from_file(path).map_err(|e| anyhow!("tokenizer {}: {e}", path.display()))
}

/// Builds a tokenizer from gguf metadata, inferring the model from what's present:
/// SentencePiece/Unigram when `scores` exist, otherwise byte-level BPE from `merges`.
pub fn from_gguf(g: &Gguf) -> Result<Tokenizer> {
	if g.f32_arr("tokenizer.ggml.scores").is_ok() {
		from_gguf_unigram(g)
	} else {
		from_gguf_bpe(g)
	}
}

fn from_gguf_unigram(g: &Gguf) -> Result<Tokenizer> {
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

fn from_gguf_bpe(g: &Gguf) -> Result<Tokenizer> {
	let pieces = g.str_arr("tokenizer.ggml.tokens")?;
	let merges = g.str_arr("tokenizer.ggml.merges")?;
	let vocab: Vocab = pieces
		.iter()
		.enumerate()
		.map(|(i, t)| (t.clone(), i as u32))
		.collect();
	let merge_pairs: Vec<(String, String)> = merges
		.iter()
		.filter_map(|m| {
			let mut it = m.splitn(2, ' ');
			let a = it.next()?;
			let b = it.next()?;
			Some((a.to_owned(), b.to_owned()))
		})
		.collect();
	let bpe = BPE::builder()
		.vocab_and_merges(vocab, merge_pairs)
		.build()
		.map_err(|e| anyhow!("bpe: {e}"))?;
	let mut tk = Tokenizer::new(bpe);
	tk.with_pre_tokenizer(Some(ByteLevel::new(false, true, true)));
	tk.with_decoder(Some(ByteLevel::new(false, true, true)));
	Ok(tk)
}

pub fn gguf_vocab(g: &Gguf, size: usize) -> Result<Vec<String>> {
	let mut v = g.str_arr("tokenizer.ggml.tokens")?;
	v.resize(size, String::new());
	Ok(v)
}
