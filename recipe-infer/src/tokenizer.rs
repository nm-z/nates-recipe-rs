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
/// SentencePiece (score-ranked BPE) when `scores` exist, otherwise byte-level BPE
/// from `merges`.
pub fn from_gguf(g: &Gguf) -> Result<Tokenizer> {
	if g.f32_arr("tokenizer.ggml.scores").is_ok() {
		from_gguf_spm(g)
	} else {
		from_gguf_bpe(g)
	}
}

/// llama.cpp's SPM tokenizer (`llm_tokenizer_spm`) is greedy bigram-BPE ranked by
/// piece SCORE — adjacent pieces merge when their concatenation is in the vocab,
/// best score first — NOT Unigram Viterbi, which segments the same text
/// differently. Reconstructed as a BPE merge table the same way HF's
/// slow-tokenizer converter does: every vocab piece that splits into two vocab
/// pieces contributes a merge, ranked by the merged piece's score (descending,
/// stable). Metaspace pre-tokenizer/decoder carries the `▁ = space` convention
/// and the leading dummy prefix; unmapped bytes fall back to `<0xXX>` pieces.
fn from_gguf_spm(g: &Gguf) -> Result<Tokenizer> {
	let pieces = g.str_arr("tokenizer.ggml.tokens")?;
	let scores = g.f32_arr("tokenizer.ggml.scores")?;
	let vocab: Vocab = pieces
		.iter()
		.enumerate()
		.map(|(i, t)| (t.clone(), i as u32))
		.collect();
	let mut merges: Vec<(f32, String, String)> = Vec::new();
	for (p, &sc) in pieces.iter().zip(scores.iter()) {
		for (b, _c) in p.char_indices().skip(1) {
			let (l, r) = p.split_at(b);
			if vocab.contains_key(l) && vocab.contains_key(r) {
				merges.push((sc, l.to_owned(), r.to_owned()));
			}
		}
	}
	merges.sort_by(|a, b| return b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
	let merge_pairs: Vec<(String, String)> =
		merges.into_iter().map(|(_sc, l, r)| return (l, r)).collect();
	let unk = g.u32_kv("tokenizer.ggml.unknown_token_id").unwrap_or(0);
	let unk_piece = pieces
		.get(unk as usize)
		.cloned()
		.unwrap_or_else(|| return "<unk>".to_owned());
	let bpe = BPE::builder()
		.vocab_and_merges(vocab, merge_pairs)
		.unk_token(unk_piece)
		.byte_fallback(true)
		.fuse_unk(true)
		.build()
		.map_err(|e| anyhow!("spm bpe: {e}"))?;
	let mut tk = Tokenizer::new(bpe);
	let ms = tokenizers::pre_tokenizers::metaspace::Metaspace::new(
		'\u{2581}',
		tokenizers::pre_tokenizers::metaspace::PrependScheme::First,
		false,
	);
	tk.with_pre_tokenizer(Some(ms));
	let decode_chain: Vec<tokenizers::DecoderWrapper> = vec![
		tokenizers::normalizers::Replace::new("\u{2581}", " ")
			.map_err(|e| anyhow!("replace: {e}"))?
			.into(),
		tokenizers::decoders::byte_fallback::ByteFallback::new().into(),
		tokenizers::decoders::fuse::Fuse::new().into(),
		tokenizers::decoders::strip::Strip::new(' ', 1, 0).into(),
	];
	tk.with_decoder(Some(tokenizers::decoders::sequence::Sequence::new(decode_chain)));
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
