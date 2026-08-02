#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]

//! Bounded host-side text metadata transformations for Recipe.
//!
//! Tokenization converts raw text into checked `int32` identifiers before the
//! single init admission. Chat rendering produces raw text before that same
//! boundary. Neither API scores tokens, transforms calculation payloads, reads
//! files during the loop, or retains an external file handle.

use core::{fmt, num::NonZeroUsize};
use std::{collections::HashSet, path::Path, sync::Arc};

use hf_chat_template::{
	ChatTemplate,
	minijinja::{Value, context},
};
use recipe_ingest::{SourceLimit, read_source_snapshot};
use tokenizers::{
	Tokenizer as InnerTokenizer,
	decoders::sequence::Sequence,
	models::bpe::{BPE, Vocab},
	pre_tokenizers::byte_level::ByteLevel,
};

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
	pub const fn model_bytes(self) -> NonZeroUsize { self.model_bytes }

	#[must_use]
	pub const fn input_bytes(self) -> NonZeroUsize { self.input_bytes }

	#[must_use]
	pub const fn output_tokens(self) -> NonZeroUsize { self.output_tokens }

	#[must_use]
	pub const fn vocabulary_entries(self) -> NonZeroUsize { self.vocabulary_entries }

	#[must_use]
	pub const fn aggregate_piece_bytes(self) -> NonZeroUsize { self.aggregate_piece_bytes }

	#[must_use]
	pub const fn merge_entries(self) -> NonZeroUsize { self.merge_entries }

	#[must_use]
	pub const fn template_bytes(self) -> NonZeroUsize { self.template_bytes }

	#[must_use]
	pub const fn messages(self) -> NonZeroUsize { self.messages }

	#[must_use]
	pub const fn rendered_bytes(self) -> NonZeroUsize { self.rendered_bytes }
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
	InvalidBatch,
	TokenizerMismatch,
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

/// Exact tokenizer configuration identity carried by prepared text artifacts.
///
/// Equality compares the complete canonical tokenizer serialization, including
/// its vocabulary, added-token metadata, normalizer, pre-tokenizer, and decoder.
/// It is intentionally not a lossy hash or a process-local instance identifier.
#[derive(Clone, PartialEq, Eq)]
pub struct TokenizerIdentity {
	canonical: Arc<str>,
}

impl fmt::Debug for TokenizerIdentity {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("TokenizerIdentity")
			.field("canonical_bytes", &self.canonical.len())
			.finish_non_exhaustive()
	}
}

/// Position at which padding is placed inside every fixed-width sequence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaddingSide {
	Left,
	Right,
}

/// Deterministic handling for tokenized inputs longer than the fixed width.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TruncationPolicy {
	/// Refuse to silently discard tokens.
	Reject,
	/// Retain the first `sequence_length` tokens and discard the tail.
	KeepStart,
	/// Retain the last `sequence_length` tokens and discard the head.
	KeepEnd,
}

/// Token identifiers with roles that preparation must retain explicitly.
///
/// Role overlap is permitted. For example, a model may deliberately use its
/// end-of-sequence token for padding. Validity is never inferred from any ID.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextTokenIds {
	pad: i32,
	unknown: Option<i32>,
	special: Box<[i32]>,
}

impl TextTokenIds {
	/// Construct checked token-role metadata.
	///
	/// # Errors
	///
	/// Returns [`TextErrorKind::InvalidTokenId`] for a negative identifier.
	pub fn new(pad: i32, unknown: Option<i32>, special: impl Into<Vec<i32>>) -> TextResult<Self> {
		let special = special.into();
		for (role, id) in core::iter::once(("pad", pad))
			.chain(unknown.map(|id| ("unknown", id)))
			.chain(special.iter().copied().map(|id| ("special", id)))
		{
			if id < 0 {
				return Err(TextError::new(
					TextErrorKind::InvalidTokenId,
					format!("{role} token ID {id} is negative"),
				));
			}
		}
		Ok(Self {
			pad,
			unknown,
			special: special.into_boxed_slice(),
		})
	}

	#[must_use]
	pub const fn pad(&self) -> i32 { self.pad }

	#[must_use]
	pub const fn unknown(&self) -> Option<i32> { self.unknown }

	#[must_use]
	pub fn special(&self) -> &[i32] { &self.special }
}

/// Immutable preparation contract for one bounded fixed-width text batch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextBatchSpec {
	max_sequences: NonZeroUsize,
	sequence_length: NonZeroUsize,
	padding: PaddingSide,
	truncation: TruncationPolicy,
	add_special_tokens: bool,
	token_ids: TextTokenIds,
}

impl TextBatchSpec {
	/// Construct a preparation contract with explicit batch and sequence bounds.
	///
	/// # Errors
	///
	/// Returns [`TextErrorKind::InvalidLimit`] when either bound is zero and
	/// [`TextErrorKind::ArithmeticOverflow`] when their maximum flat layout does
	/// not fit `usize`.
	pub fn new(
		max_sequences: usize,
		sequence_length: usize,
		padding: PaddingSide,
		truncation: TruncationPolicy,
		add_special_tokens: bool,
		token_ids: TextTokenIds,
	) -> TextResult<Self> {
		let max_sequences = nonzero("text batch sequences", max_sequences)?;
		let sequence_length = nonzero("text sequence length", sequence_length)?;
		max_sequences
			.get()
			.checked_mul(sequence_length.get())
			.ok_or_else(|| {
				TextError::new(
					TextErrorKind::ArithmeticOverflow,
					"maximum text batch layout overflows usize",
				)
			})?;
		Ok(Self {
			max_sequences,
			sequence_length,
			padding,
			truncation,
			add_special_tokens,
			token_ids,
		})
	}

	#[must_use]
	pub const fn max_sequences(&self) -> NonZeroUsize { self.max_sequences }

	#[must_use]
	pub const fn sequence_length(&self) -> NonZeroUsize { self.sequence_length }

	#[must_use]
	pub const fn padding(&self) -> PaddingSide { self.padding }

	#[must_use]
	pub const fn truncation(&self) -> TruncationPolicy { self.truncation }

	#[must_use]
	pub const fn add_special_tokens(&self) -> bool { self.add_special_tokens }

	#[must_use]
	pub const fn token_ids(&self) -> &TextTokenIds { &self.token_ids }
}

/// Stable physical layout of a prepared text batch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextBatchLayout {
	/// Contiguous `[batch, sequence]` rows; flat index is
	/// `batch_index * sequence_length + sequence_index`.
	BatchMajor,
}

/// Immutable, fixed-width token and attention metadata produced at preparation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextBatch {
	tokenizer: TokenizerIdentity,
	spec: TextBatchSpec,
	sequences: usize,
	token_ids: Box<[i32]>,
	attention_mask: Box<[i32]>,
	original_lengths: Box<[usize]>,
	retained_lengths: Box<[usize]>,
}

impl TextBatch {
	#[must_use]
	pub const fn layout(&self) -> TextBatchLayout { TextBatchLayout::BatchMajor }

	#[must_use]
	pub const fn tokenizer_identity(&self) -> &TokenizerIdentity { &self.tokenizer }

	#[must_use]
	pub const fn spec(&self) -> &TextBatchSpec { &self.spec }

	#[must_use]
	pub const fn sequences(&self) -> usize { self.sequences }

	#[must_use]
	pub fn shape(&self) -> [usize; 2] { [self.sequences, self.spec.sequence_length.get()] }

	/// Flat batch-major token storage, including explicit padding positions.
	#[must_use]
	pub fn token_ids(&self) -> &[i32] { &self.token_ids }

	/// Flat batch-major validity mask containing only exact zero and one.
	///
	/// A value of one marks a retained encoded token. A value of zero marks
	/// padding, even when a retained token happens to equal the pad token ID.
	#[must_use]
	pub fn attention_mask(&self) -> &[i32] { &self.attention_mask }

	#[must_use]
	pub fn original_lengths(&self) -> &[usize] { &self.original_lengths }

	#[must_use]
	pub fn retained_lengths(&self) -> &[usize] { &self.retained_lengths }

	/// Return one complete fixed-width batch row.
	#[must_use]
	pub fn row(&self, index: usize) -> Option<TextBatchRow<'_>> {
		let width = self.spec.sequence_length.get();
		let start = index.checked_mul(width)?;
		let end = start.checked_add(width)?;
		Some(TextBatchRow {
			token_ids: self.token_ids.get(start..end)?,
			attention_mask: self.attention_mask.get(start..end)?,
			original_length: *self.original_lengths.get(index)?,
			retained_length: *self.retained_lengths.get(index)?,
		})
	}
}

/// Borrowed view of one fixed-width batch row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextBatchRow<'a> {
	token_ids: &'a [i32],
	attention_mask: &'a [i32],
	original_length: usize,
	retained_length: usize,
}

impl<'a> TextBatchRow<'a> {
	#[must_use]
	pub const fn token_ids(self) -> &'a [i32] { self.token_ids }

	#[must_use]
	pub const fn attention_mask(self) -> &'a [i32] { self.attention_mask }

	#[must_use]
	pub const fn original_length(self) -> usize { self.original_length }

	#[must_use]
	pub const fn retained_length(self) -> usize { self.retained_length }
}

/// A preparation-owned tokenizer with immutable invocation limits.
pub struct Tokenizer {
	inner: InnerTokenizer,
	limits: TextLimits,
	identity: TokenizerIdentity,
}

impl fmt::Debug for Tokenizer {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("Tokenizer")
			.field("vocabulary_size", &self.inner.get_vocab_size(true))
			.field("limits", &self.limits)
			.field("identity", &self.identity)
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
		let identity = tokenizer_identity(&inner)?;
		Ok(Self {
			inner,
			limits,
			identity,
		})
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
		let identity = tokenizer_identity(&inner)?;
		Ok(Self {
			inner,
			limits,
			identity,
		})
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
	pub fn vocabulary_size(&self) -> usize { self.inner.get_vocab_size(true) }

	/// Exact tokenizer identity attached to every prepared batch.
	#[must_use]
	pub const fn identity(&self) -> &TokenizerIdentity { &self.identity }

	/// Encode and deterministically lay out a bounded fixed-width batch.
	///
	/// The returned attention mask is constructed from retained positions. Pad,
	/// unknown, and special token values never determine validity.
	///
	/// # Errors
	///
	/// Rejects an empty batch, exceeded bounds, undeclared-vocabulary role IDs,
	/// tokenization failures, rejected truncation, and layout overflow.
	pub fn prepare_batch(&self, texts: &[&str], spec: &TextBatchSpec) -> TextResult<TextBatch> {
		if texts.is_empty() {
			return Err(TextError::new(
				TextErrorKind::InvalidBatch,
				"text batch contains no sequences",
			));
		}
		require_limit(
			"text batch sequences",
			texts.len(),
			spec.max_sequences.get(),
		)?;
		require_limit(
			"fixed text sequence length",
			spec.sequence_length.get(),
			self.limits.output_tokens.get(),
		)?;
		self.validate_token_roles(spec.token_ids())?;

		let width = spec.sequence_length.get();
		let elements = texts.len().checked_mul(width).ok_or_else(|| {
			TextError::new(
				TextErrorKind::ArithmeticOverflow,
				"prepared text batch layout overflows usize",
			)
		})?;
		let mut token_ids = vec![spec.token_ids.pad; elements];
		let mut attention_mask = vec![0_i32; elements];
		let mut original_lengths = Vec::with_capacity(texts.len());
		let mut retained_lengths = Vec::with_capacity(texts.len());

		for (batch_index, text) in texts.iter().enumerate() {
			let encoded = self.encode(text, spec.add_special_tokens)?;
			let original_length = encoded.len();
			let retained = if original_length <= width {
				encoded.as_slice()
			} else {
				match spec.truncation {
					TruncationPolicy::Reject => {
						return Err(TextError::new(
							TextErrorKind::LimitExceeded,
							format!(
								"tokenized sequence {batch_index} has {original_length} tokens, fixed length is {width}"
							),
						));
					}
					TruncationPolicy::KeepStart => &encoded[..width],
					TruncationPolicy::KeepEnd => &encoded[original_length - width..],
				}
			};
			let retained_length = retained.len();
			let row_start = batch_index.checked_mul(width).ok_or_else(|| {
				TextError::new(
					TextErrorKind::ArithmeticOverflow,
					"prepared text row offset overflows usize",
				)
			})?;
			let token_start = match spec.padding {
				PaddingSide::Left => row_start + width - retained_length,
				PaddingSide::Right => row_start,
			};
			let token_end = token_start + retained_length;
			token_ids[token_start..token_end].copy_from_slice(retained);
			attention_mask[token_start..token_end].fill(1);
			original_lengths.push(original_length);
			retained_lengths.push(retained_length);
		}

		Ok(TextBatch {
			tokenizer: self.identity.clone(),
			spec: spec.clone(),
			sequences: texts.len(),
			token_ids: token_ids.into_boxed_slice(),
			attention_mask: attention_mask.into_boxed_slice(),
			original_lengths: original_lengths.into_boxed_slice(),
			retained_lengths: retained_lengths.into_boxed_slice(),
		})
	}

	/// Decode one prepared row using the tokenizer that gave its IDs meaning.
	///
	/// Padding is removed exclusively through the stored validity mask. A valid
	/// token equal to the pad ID remains in the decoded sequence.
	///
	/// # Errors
	///
	/// Returns [`TextErrorKind::TokenizerMismatch`] for a batch produced by a
	/// different tokenizer configuration and [`TextErrorKind::InvalidBatch`] for
	/// an out-of-range row.
	pub fn decode_batch_row(&self, batch: &TextBatch, row: usize, skip_special_tokens: bool) -> TextResult<String> {
		if self.identity != batch.tokenizer {
			return Err(TextError::new(
				TextErrorKind::TokenizerMismatch,
				"prepared batch tokenizer identity does not match decoder",
			));
		}
		let row = batch.row(row).ok_or_else(|| {
			TextError::new(
				TextErrorKind::InvalidBatch,
				format!("prepared text batch row {row} is out of range"),
			)
		})?;
		let retained = row
			.token_ids()
			.iter()
			.zip(row.attention_mask())
			.filter_map(|(id, valid)| (*valid == 1).then_some(*id))
			.collect::<Vec<_>>();
		self.decode(&retained, skip_special_tokens)
	}

	fn validate_token_roles(&self, roles: &TextTokenIds) -> TextResult<()> {
		for (role, id) in core::iter::once(("pad", roles.pad))
			.chain(roles.unknown.map(|id| ("unknown", id)))
			.chain(roles.special.iter().copied().map(|id| ("special", id)))
		{
			let unsigned = u32::try_from(id).map_err(|error| {
				TextError::new(
					TextErrorKind::InvalidTokenId,
					format!("{role} token ID {id} is negative: {error}"),
				)
			})?;
			if self.inner.id_to_token(unsigned).is_none() {
				return Err(TextError::new(
					TextErrorKind::InvalidTokenId,
					format!("{role} token ID {id} is outside this tokenizer vocabulary"),
				));
			}
		}
		Ok(())
	}
}

fn tokenizer_identity(inner: &InnerTokenizer) -> TextResult<TokenizerIdentity> {
	let canonical = inner
		.to_string(false)
		.map_err(|error| TextError::new(TextErrorKind::InvalidModel, error.to_string()))?;
	Ok(TokenizerIdentity {
		canonical: Arc::from(canonical),
	})
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
		VocabularyKind::BytePair if !spec.score_bits.is_empty() => {
			Err(TextError::new(
				TextErrorKind::InvalidVocabulary,
				"byte-pair vocabulary unexpectedly contains SentencePiece scores",
			))
		}
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
