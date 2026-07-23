#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]

//! Bounded host-side text metadata transformations for Recipe.
//!
//! Tokenization converts raw text into checked `int32` identifiers before the
//! single init admission. Chat rendering produces raw text before that same
//! boundary. Neither API scores tokens, transforms calculation payloads, reads
//! files during the loop, or retains an external file handle.

use core::fmt;
use core::num::NonZeroUsize;
use std::collections::HashSet;
use std::path::Path;

use hf_chat_template::ChatTemplate;
use hf_chat_template::minijinja::{Value, context};
use recipe_ingest::{SourceLimit, read_source_snapshot};
use tokenizers::Tokenizer as InnerTokenizer;
use tokenizers::decoders::sequence::Sequence;
use tokenizers::models::bpe::{BPE, Vocab};
use tokenizers::pre_tokenizers::byte_level::ByteLevel;

/// Aggregate limits applied while constructing and invoking one tokenizer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextLimits {
	model_bytes: NonZeroUsize,
	input_bytes: NonZeroUsize,
	output_tokens: NonZeroUsize,
	vocabulary_entries: NonZeroUsize,
	aggregate_piece_bytes: NonZeroUsize,
	merge_entries: NonZeroUsize,
	template_bytes: NonZeroUsize,
	messages: NonZeroUsize,
	rendered_bytes: NonZeroUsize,
}

impl TextLimits {
	/// Construct nonzero bounds for every externally sized text surface.
	///
	/// # Errors
	///
	/// Returns [`TextErrorKind::InvalidLimit`] when any bound is zero.
	#[allow(clippy::too_many_arguments)]
	pub fn new(
		model_bytes: usize,
		input_bytes: usize,
		output_tokens: usize,
		vocabulary_entries: usize,
		aggregate_piece_bytes: usize,
		merge_entries: usize,
		template_bytes: usize,
		messages: usize,
		rendered_bytes: usize,
	) -> TextResult<Self> {
		Ok(Self {
			model_bytes: nonzero("tokenizer model bytes", model_bytes)?,
			input_bytes: nonzero("input bytes", input_bytes)?,
			output_tokens: nonzero("output tokens", output_tokens)?,
			vocabulary_entries: nonzero("vocabulary entries", vocabulary_entries)?,
			aggregate_piece_bytes: nonzero("aggregate piece bytes", aggregate_piece_bytes)?,
			merge_entries: nonzero("merge entries", merge_entries)?,
			template_bytes: nonzero("template bytes", template_bytes)?,
			messages: nonzero("messages", messages)?,
			rendered_bytes: nonzero("rendered bytes", rendered_bytes)?,
		})
	}

	#[must_use]
	pub const fn model_bytes(self) -> NonZeroUsize {
		self.model_bytes
	}

	#[must_use]
	pub const fn input_bytes(self) -> NonZeroUsize {
		self.input_bytes
	}

	#[must_use]
	pub const fn output_tokens(self) -> NonZeroUsize {
		self.output_tokens
	}

	#[must_use]
	pub const fn vocabulary_entries(self) -> NonZeroUsize {
		self.vocabulary_entries
	}

	#[must_use]
	pub const fn aggregate_piece_bytes(self) -> NonZeroUsize {
		self.aggregate_piece_bytes
	}

	#[must_use]
	pub const fn merge_entries(self) -> NonZeroUsize {
		self.merge_entries
	}

	#[must_use]
	pub const fn template_bytes(self) -> NonZeroUsize {
		self.template_bytes
	}

	#[must_use]
	pub const fn messages(self) -> NonZeroUsize {
		self.messages
	}

	#[must_use]
	pub const fn rendered_bytes(self) -> NonZeroUsize {
		self.rendered_bytes
	}
}

fn nonzero(name: &str, value: usize) -> TextResult<NonZeroUsize> {
	NonZeroUsize::new(value).ok_or_else(|| {
		TextError::new(
			TextErrorKind::InvalidLimit,
			format!("{name} limit must be nonzero"),
		)
	})
}

/// Tokenizer construction flavor used by GGUF metadata adapters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VocabularyKind {
	BytePair,
	SentencePieceBpe,
}

/// Borrowed, already framed vocabulary metadata.
#[derive(Clone, Copy, Debug)]
pub struct VocabularySpec<'a> {
	pub kind: VocabularyKind,
	pub tokens: &'a [String],
	pub merges: &'a [String],
	/// IEEE-754 f32 bits, preserving GGUF metadata without host arithmetic.
	pub score_bits: &'a [u32],
	pub unknown_token_id: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TextErrorKind {
	InvalidLimit,
	LimitExceeded,
	InvalidModel,
	InvalidVocabulary,
	InvalidTokenId,
	Tokenization,
	Decode,
	InvalidMessage,
	Template,
	Source,
	ArithmeticOverflow,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextError {
	pub kind: TextErrorKind,
	pub detail: String,
}

impl TextError {
	fn new(kind: TextErrorKind, detail: impl Into<String>) -> Self {
		Self {
			kind,
			detail: detail.into(),
		}
	}
}

impl fmt::Display for TextError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(formatter, "{:?}: {}", self.kind, self.detail)
	}
}

impl std::error::Error for TextError {}

pub type TextResult<T> = Result<T, TextError>;

/// A preparation-owned tokenizer with immutable invocation limits.
pub struct Tokenizer {
	inner: InnerTokenizer,
	limits: TextLimits,
}

impl fmt::Debug for Tokenizer {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("Tokenizer")
			.field("vocabulary_size", &self.inner.get_vocab_size(true))
			.field("limits", &self.limits)
			.finish_non_exhaustive()
	}
}

impl Tokenizer {
	/// Parse a bounded Hugging Face `tokenizer.json` snapshot.
	///
	/// # Errors
	///
	/// Rejects oversized or malformed models.
	pub fn from_json(bytes: &[u8], limits: TextLimits) -> TextResult<Self> {
		require_limit(
			"tokenizer model bytes",
			bytes.len(),
			limits.model_bytes.get(),
		)?;
		let inner = InnerTokenizer::from_bytes(bytes)
			.map_err(|error| TextError::new(TextErrorKind::InvalidModel, error.to_string()))?;
		validate_vocab_size(&inner, limits)?;
		Ok(Self { inner, limits })
	}

	/// Snapshot and parse one tokenizer file during preparation.
	///
	/// # Errors
	///
	/// Rejects source, size, or tokenizer-format failures.
	pub fn from_file(path: &Path, limits: TextLimits) -> TextResult<Self> {
		let byte_limit = u64::try_from(limits.model_bytes.get()).map_err(|error| {
			TextError::new(
				TextErrorKind::ArithmeticOverflow,
				format!("tokenizer model limit does not fit u64: {error}"),
			)
		})?;
		let source_limit = SourceLimit::new(byte_limit)
			.map_err(|error| TextError::new(TextErrorKind::Source, error.to_string()))?;
		let snapshot = read_source_snapshot(path, source_limit)
			.map_err(|error| TextError::new(TextErrorKind::Source, error.to_string()))?;
		Self::from_json(snapshot.bytes(), limits)
	}

	/// Construct the exact BPE flavor declared by already validated model
	/// metadata.
	///
	/// # Errors
	///
	/// Rejects duplicate/oversized pieces, malformed merges or scores, and
	/// token identifiers outside the calculation contract's `int32` domain.
	pub fn from_vocabulary(spec: VocabularySpec<'_>, limits: TextLimits) -> TextResult<Self> {
		validate_vocabulary(spec, limits)?;
		let vocab = vocabulary_map(spec.tokens)?;
		let inner = match spec.kind {
			VocabularyKind::BytePair => byte_pair_tokenizer(vocab, spec.merges)?,
			VocabularyKind::SentencePieceBpe => {
				sentencepiece_bpe_tokenizer(vocab, spec.tokens, spec.score_bits, spec.unknown_token_id)?
			}
		};
		validate_vocab_size(&inner, limits)?;
		Ok(Self { inner, limits })
	}

	/// Convert raw text to checked `int32` token metadata.
	///
	/// # Errors
	///
	/// Rejects oversized input/output, tokenizer failures, or identifiers above
	/// `i32::MAX`.
	pub fn encode(&self, text: &str, add_special_tokens: bool) -> TextResult<Vec<i32>> {
		require_limit(
			"tokenizer input bytes",
			text.len(),
			self.limits.input_bytes.get(),
		)?;
		let encoded = self
			.inner
			.encode(text, add_special_tokens)
			.map_err(|error| TextError::new(TextErrorKind::Tokenization, error.to_string()))?;
		require_limit(
			"tokenizer output tokens",
			encoded.len(),
			self.limits.output_tokens.get(),
		)?;
		encoded
			.get_ids()
			.iter()
			.copied()
			.map(|id| {
				i32::try_from(id).map_err(|error| {
					TextError::new(
						TextErrorKind::InvalidTokenId,
						format!("token ID {id} is outside int32: {error}"),
					)
				})
			})
			.collect()
	}

	/// Decode checked token metadata to raw text.
	///
	/// # Errors
	///
	/// Rejects negative IDs, oversized token sequences, decoder failures, or
	/// output above the configured byte bound.
	pub fn decode(&self, ids: &[i32], skip_special_tokens: bool) -> TextResult<String> {
		require_limit(
			"decoder input tokens",
			ids.len(),
			self.limits.output_tokens.get(),
		)?;
		let ids = ids
			.iter()
			.copied()
			.map(|id| {
				u32::try_from(id).map_err(|error| {
					TextError::new(
						TextErrorKind::InvalidTokenId,
						format!("token ID {id} is negative: {error}"),
					)
				})
			})
			.collect::<TextResult<Vec<_>>>()?;
		let decoded = self
			.inner
			.decode(&ids, skip_special_tokens)
			.map_err(|error| TextError::new(TextErrorKind::Decode, error.to_string()))?;
		require_limit(
			"decoded output bytes",
			decoded.len(),
			self.limits.rendered_bytes.get(),
		)?;
		Ok(decoded)
	}

	#[must_use]
	pub fn vocabulary_size(&self) -> usize {
		self.inner.get_vocab_size(true)
	}
}

fn validate_vocab_size(inner: &InnerTokenizer, limits: TextLimits) -> TextResult<()> {
	let size = inner.get_vocab_size(true);
	require_limit(
		"tokenizer vocabulary entries",
		size,
		limits.vocabulary_entries.get(),
	)?;
	if size > i32::MAX as usize {
		return Err(TextError::new(
			TextErrorKind::InvalidTokenId,
			format!("tokenizer vocabulary has {size} entries, exceeding int32"),
		));
	}
	Ok(())
}

fn validate_vocabulary(spec: VocabularySpec<'_>, limits: TextLimits) -> TextResult<()> {
	if spec.tokens.is_empty() {
		return Err(TextError::new(
			TextErrorKind::InvalidVocabulary,
			"token vocabulary is empty",
		));
	}
	require_limit(
		"token vocabulary entries",
		spec.tokens.len(),
		limits.vocabulary_entries.get(),
	)?;
	require_limit(
		"token merge entries",
		spec.merges.len(),
		limits.merge_entries.get(),
	)?;
	let piece_bytes = spec.tokens.iter().try_fold(0_usize, |total, token| {
		total.checked_add(token.len()).ok_or_else(|| {
			TextError::new(
				TextErrorKind::ArithmeticOverflow,
				"aggregate token piece bytes overflowed usize",
			)
		})
	})?;
	require_limit(
		"aggregate token piece bytes",
		piece_bytes,
		limits.aggregate_piece_bytes.get(),
	)?;
	if spec.tokens.len() > i32::MAX as usize {
		return Err(TextError::new(
			TextErrorKind::InvalidTokenId,
			"token vocabulary exceeds int32 identifiers",
		));
	}
	let mut unique = HashSet::with_capacity(spec.tokens.len());
	for token in spec.tokens {
		if !unique.insert(token.as_str()) {
			return Err(TextError::new(
				TextErrorKind::InvalidVocabulary,
				format!("duplicate token piece {token:?}"),
			));
		}
	}
	match spec.kind {
		VocabularyKind::BytePair if !spec.score_bits.is_empty() => Err(TextError::new(
			TextErrorKind::InvalidVocabulary,
			"byte-pair vocabulary unexpectedly contains SentencePiece scores",
		)),
		VocabularyKind::SentencePieceBpe
			if spec.score_bits.len() != spec.tokens.len() || !spec.merges.is_empty() =>
		{
			Err(TextError::new(
				TextErrorKind::InvalidVocabulary,
				"SentencePiece BPE requires one score per token and no explicit merge table",
			))
		}
		VocabularyKind::SentencePieceBpe => {
			for bits in spec.score_bits {
				let score = f32::from_bits(*bits);
				if !score.is_finite() {
					return Err(TextError::new(
						TextErrorKind::InvalidVocabulary,
						"SentencePiece score is not finite",
					));
				}
			}
			let unknown = spec.unknown_token_id.ok_or_else(|| {
				TextError::new(
					TextErrorKind::InvalidVocabulary,
					"SentencePiece BPE requires an unknown token ID",
				)
			})?;
			if usize::try_from(unknown).map_or(true, |index| index >= spec.tokens.len()) {
				return Err(TextError::new(
					TextErrorKind::InvalidTokenId,
					format!("unknown token ID {unknown} is outside the vocabulary"),
				));
			}
			Ok(())
		}
		VocabularyKind::BytePair => Ok(()),
	}
}

fn vocabulary_map(tokens: &[String]) -> TextResult<Vocab> {
	tokens.iter()
		.enumerate()
		.map(|(index, token)| {
			let id = u32::try_from(index).map_err(|error| {
				TextError::new(
					TextErrorKind::InvalidTokenId,
					format!("token index {index} does not fit u32: {error}"),
				)
			})?;
			Ok((token.clone(), id))
		})
		.collect()
}

fn byte_pair_tokenizer(vocab: Vocab, merges: &[String]) -> TextResult<InnerTokenizer> {
	let merge_pairs = merges
		.iter()
		.map(|merge| {
			let mut pieces = merge.split(' ');
			let left = pieces.next().unwrap_or_default();
			let right = pieces.next().unwrap_or_default();
			if left.is_empty() || right.is_empty() || pieces.next().is_some() {
				return Err(TextError::new(
					TextErrorKind::InvalidVocabulary,
					format!("BPE merge {merge:?} is not exactly two space-separated pieces"),
				));
			}
			Ok((left.to_owned(), right.to_owned()))
		})
		.collect::<TextResult<Vec<_>>>()?;
	let bpe = BPE::builder()
		.vocab_and_merges(vocab, merge_pairs)
		.build()
		.map_err(|error| TextError::new(TextErrorKind::InvalidVocabulary, error.to_string()))?;
	let byte_level = ByteLevel::new(false, true, true);
	let mut tokenizer = InnerTokenizer::new(bpe);
	tokenizer.with_pre_tokenizer(Some(byte_level));
	tokenizer.with_decoder(Some(byte_level));
	Ok(tokenizer)
}

fn sentencepiece_bpe_tokenizer(
	vocab: Vocab,
	tokens: &[String],
	score_bits: &[u32],
	unknown_token_id: Option<u32>,
) -> TextResult<InnerTokenizer> {
	let mut merge_candidates = Vec::new();
	for (token, bits) in tokens.iter().zip(score_bits) {
		let score = f32::from_bits(*bits);
		for boundary in token.char_indices().skip(1).map(|(index, _)| index) {
			let (left, right) = token.split_at(boundary);
			if vocab.contains_key(left) && vocab.contains_key(right) {
				merge_candidates.push((score, left.to_owned(), right.to_owned()));
			}
		}
	}
	merge_candidates.sort_by(|left, right| {
		right.0
			.total_cmp(&left.0)
			.then_with(|| left.1.cmp(&right.1))
			.then_with(|| left.2.cmp(&right.2))
	});
	let merges = merge_candidates
		.into_iter()
		.map(|(_, left, right)| (left, right))
		.collect::<Vec<_>>();
	let unknown = usize::try_from(unknown_token_id.expect("validated unknown token ID"))
		.expect("u32 fits usize on supported targets");
	let unknown_piece = tokens
		.get(unknown)
		.expect("validated unknown token ID")
		.clone();
	let bpe = BPE::builder()
		.vocab_and_merges(vocab, merges)
		.unk_token(unknown_piece)
		.byte_fallback(true)
		.fuse_unk(true)
		.build()
		.map_err(|error| TextError::new(TextErrorKind::InvalidVocabulary, error.to_string()))?;
	let metaspace = tokenizers::pre_tokenizers::metaspace::Metaspace::new(
		'\u{2581}',
		tokenizers::pre_tokenizers::metaspace::PrependScheme::First,
		false,
	);
	let decoder_chain: Vec<tokenizers::DecoderWrapper> = vec![
		tokenizers::normalizers::Replace::new("\u{2581}", " ")
			.map_err(|error| TextError::new(TextErrorKind::InvalidVocabulary, error.to_string()))?
			.into(),
		tokenizers::decoders::byte_fallback::ByteFallback::new().into(),
		tokenizers::decoders::fuse::Fuse::new().into(),
		tokenizers::decoders::strip::Strip::new(' ', 1, 0).into(),
	];
	let mut tokenizer = InnerTokenizer::new(bpe);
	tokenizer.with_pre_tokenizer(Some(metaspace));
	tokenizer.with_decoder(Some(Sequence::new(decoder_chain)));
	Ok(tokenizer)
}

fn require_limit(name: &str, actual: usize, limit: usize) -> TextResult<()> {
	if actual > limit {
		Err(TextError::new(
			TextErrorKind::LimitExceeded,
			format!("{name} is {actual}, limit is {limit}"),
		))
	} else {
		Ok(())
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
	pub role: String,
	pub content: String,
}

impl Message {
	#[must_use]
	pub fn new(role: impl Into<String>, content: impl Into<String>) -> Self {
		Self {
			role: role.into(),
			content: content.into(),
		}
	}
}

/// Render a bounded Hugging Face chat template before tokenization.
///
/// # Errors
///
/// Rejects oversized/empty inputs, compilation/render failures, or output
/// above the configured bound.
pub fn render_template(
	template: &str,
	messages: &[Message],
	add_generation_prompt: bool,
	bos_token: &str,
	eos_token: &str,
	limits: TextLimits,
) -> TextResult<String> {
	require_limit(
		"chat template bytes",
		template.len(),
		limits.template_bytes.get(),
	)?;
	require_limit("chat message count", messages.len(), limits.messages.get())?;
	if messages
		.iter()
		.any(|message| message.role.is_empty() || message.content.len() > limits.input_bytes.get())
	{
		return Err(TextError::new(
			TextErrorKind::InvalidMessage,
			"chat roles must be nonempty and each content must fit the input byte bound",
		));
	}
	let values = messages
		.iter()
		.map(|message| {
			context! {
				role => message.role.as_str(),
				content => message.content.as_str()
			}
		})
		.collect::<Vec<Value>>();
	let compiled = ChatTemplate::from_str(template)
		.map_err(|error| TextError::new(TextErrorKind::Template, format!("compile: {error}")))?;
	let rendered = compiled
		.render_value(context! {
			messages => values,
			add_generation_prompt => add_generation_prompt,
			bos_token => bos_token,
			eos_token => eos_token,
		})
		.map_err(|error| TextError::new(TextErrorKind::Template, format!("render: {error}")))?;
	require_limit(
		"rendered chat bytes",
		rendered.len(),
		limits.rendered_bytes.get(),
	)?;
	Ok(rendered)
}

#[cfg(test)]
mod tests {
	use super::*;

	fn limits() -> TextLimits {
		TextLimits::new(4096, 256, 64, 1024, 8192, 1024, 4096, 16, 4096).unwrap()
	}

	#[test]
	fn byte_pair_vocabulary_emits_checked_int32_ids() {
		let tokens = vec!["H".to_owned(), "i".to_owned(), "Hi".to_owned()];
		let merges = vec!["H i".to_owned()];
		let tokenizer = Tokenizer::from_vocabulary(
			VocabularySpec {
				kind: VocabularyKind::BytePair,
				tokens: &tokens,
				merges: &merges,
				score_bits: &[],
				unknown_token_id: None,
			},
			limits(),
		)
		.unwrap();
		assert_eq!(tokenizer.encode("Hi", false).unwrap(), [2]);
		assert_eq!(tokenizer.decode(&[2], false).unwrap(), "Hi");
	}

	#[test]
	fn token_and_render_bounds_fail_closed() {
		let tokens = vec!["a".to_owned()];
		let tokenizer = Tokenizer::from_vocabulary(
			VocabularySpec {
				kind: VocabularyKind::BytePair,
				tokens: &tokens,
				merges: &[],
				score_bits: &[],
				unknown_token_id: None,
			},
			limits(),
		)
		.unwrap();
		let long = "x".repeat(limits().input_bytes.get() + 1);
		assert_eq!(
			tokenizer.encode(&long, false).unwrap_err().kind,
			TextErrorKind::LimitExceeded
		);
		assert_eq!(
			tokenizer.decode(&[-1], false).unwrap_err().kind,
			TextErrorKind::InvalidTokenId
		);
		assert_eq!(
			render_template(
				"{{ messages[0].content }}",
				&[Message::new("", "x")],
				false,
				"",
				"",
				limits(),
			)
			.unwrap_err()
			.kind,
			TextErrorKind::InvalidMessage
		);
	}

	#[test]
	fn chat_template_supports_generation_prompt_metadata() {
		let rendered = render_template(
			"{% for message in messages %}{{ message.role }}:{{ message.content }}\\n{% endfor %}{% if add_generation_prompt %}assistant:{% endif %}",
			&[Message::new("user", "hello")],
			true,
			"",
			"",
			limits(),
		)
		.unwrap();
		assert_eq!(rendered, "user:hello\\nassistant:");
	}

	#[test]
	fn bounded_json_models_cover_shipped_bpe_wordpiece_and_unigram_specs() {
		for bytes in [
			include_bytes!("../../examples/datasets/tokenizers/gpt2/tokenizer.json").as_slice(),
			include_bytes!("../../examples/datasets/tokenizers/bert-base-uncased/tokenizer.json").as_slice(),
			include_bytes!("../../examples/datasets/tokenizers/t5-small/tokenizer.json").as_slice(),
		] {
			let mut model_limits = limits();
			model_limits.model_bytes = NonZeroUsize::new(bytes.len()).unwrap();
			model_limits.vocabulary_entries = NonZeroUsize::new(100_000).unwrap();
			model_limits.aggregate_piece_bytes = NonZeroUsize::new(16 * 1024 * 1024).unwrap();
			model_limits.merge_entries = NonZeroUsize::new(100_000).unwrap();
			let tokenizer = Tokenizer::from_json(bytes, model_limits).unwrap();
			let ids = tokenizer.encode("Hello world", true).unwrap();
			assert!(!ids.is_empty());
			assert!(ids.iter().all(|id| *id >= 0));
			assert!(!tokenizer.decode(&ids, false).unwrap().is_empty());
		}
	}

	#[test]
	fn vocabulary_validation_rejects_duplicates_and_nonfinite_scores() {
		let duplicate = vec!["x".to_owned(), "x".to_owned()];
		assert_eq!(
			Tokenizer::from_vocabulary(
				VocabularySpec {
					kind: VocabularyKind::BytePair,
					tokens: &duplicate,
					merges: &[],
					score_bits: &[],
					unknown_token_id: None,
				},
				limits(),
			)
			.unwrap_err()
			.kind,
			TextErrorKind::InvalidVocabulary
		);
		let tokens = vec!["<unk>".to_owned()];
		assert_eq!(
			Tokenizer::from_vocabulary(
				VocabularySpec {
					kind: VocabularyKind::SentencePieceBpe,
					tokens: &tokens,
					merges: &[],
					score_bits: &[f32::NAN.to_bits()],
					unknown_token_id: Some(0),
				},
				limits(),
			)
			.unwrap_err()
			.kind,
			TextErrorKind::InvalidVocabulary
		);
	}
}
