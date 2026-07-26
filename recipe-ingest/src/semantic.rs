use core::fmt;
use std::collections::BTreeSet;

use crate::image_header::has_recognized_image_signature;
use crate::{RawTable, parse_contract_f32, parse_contract_i32};

/// Recipe's exhaustive semantic classification for digitally stored,
/// learnable vectors.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SemanticType {
	Numeric,
	Temporal,
	Categorical,
	Ordinal,
	Text,
	Image,
	Binary,
}

/// Smallest calculation-facing representation that preserves the source
/// vector under its inferred semantic interpretation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VectorEncoding {
	F32,
	I32,
	RelativeSecondsI32,
	DictionaryI32,
	OrdinalI32,
	Utf8,
	Bytes,
}

/// Evidence presented to an ambiguous-vector classifier.
///
/// Counts are exact. Ratios use thousandths so classification is
/// deterministic without host floating-point behavior.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VectorEvidence {
	pub values: usize,
	pub missing: usize,
	pub unique: usize,
	pub utf8_values: usize,
	pub whitespace_values: usize,
	pub source_bytes: usize,
	pub dictionary_bytes: usize,
	pub mean_value_bytes: usize,
	pub unique_per_thousand: u16,
	pub whitespace_per_thousand: u16,
}

/// Model boundary for vectors that cannot be identified by a lossless
/// semantic parser.
pub trait AmbiguousVectorModel {
	fn classify(&self, evidence: VectorEvidence) -> SemanticType;
}

/// Fixed, auditable nearest-example model used by automatic table loading.
///
/// The examples distinguish labels from prose using cardinality, whitespace,
/// mean width, and whether dictionary encoding is the smaller lossless
/// representation. The model never invents a derived feature.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CategoricalEncodingModel;

#[derive(Clone, Copy)]
struct TrainingExample {
	unique_per_thousand: u16,
	whitespace_per_thousand: u16,
	mean_value_bytes: usize,
	dictionary_smaller: bool,
	label: SemanticType,
}

const AMBIGUOUS_TRAINING_EXAMPLES: [TrainingExample; 7] = [
	TrainingExample {
		unique_per_thousand: 20,
		whitespace_per_thousand: 0,
		mean_value_bytes: 5,
		dictionary_smaller: true,
		label: SemanticType::Categorical,
	},
	TrainingExample {
		unique_per_thousand: 200,
		whitespace_per_thousand: 0,
		mean_value_bytes: 12,
		dictionary_smaller: true,
		label: SemanticType::Categorical,
	},
	TrainingExample {
		unique_per_thousand: 1_000,
		whitespace_per_thousand: 0,
		mean_value_bytes: 10,
		dictionary_smaller: false,
		label: SemanticType::Categorical,
	},
	TrainingExample {
		unique_per_thousand: 600,
		whitespace_per_thousand: 0,
		mean_value_bytes: 8,
		dictionary_smaller: false,
		label: SemanticType::Categorical,
	},
	TrainingExample {
		unique_per_thousand: 800,
		whitespace_per_thousand: 700,
		mean_value_bytes: 48,
		dictionary_smaller: false,
		label: SemanticType::Text,
	},
	TrainingExample {
		unique_per_thousand: 1_000,
		whitespace_per_thousand: 900,
		mean_value_bytes: 120,
		dictionary_smaller: false,
		label: SemanticType::Text,
	},
	TrainingExample {
		unique_per_thousand: 700,
		whitespace_per_thousand: 300,
		mean_value_bytes: 32,
		dictionary_smaller: false,
		label: SemanticType::Text,
	},
];

impl AmbiguousVectorModel for CategoricalEncodingModel {
	fn classify(&self, evidence: VectorEvidence) -> SemanticType {
		if evidence.utf8_values != evidence.values.saturating_sub(evidence.missing) {
			return SemanticType::Categorical;
		}
		let dictionary_smaller = evidence.dictionary_bytes <= evidence.source_bytes;
		AMBIGUOUS_TRAINING_EXAMPLES
			.iter()
			.min_by_key(|example| {
				let unique = u64::from(
					example
						.unique_per_thousand
						.abs_diff(evidence.unique_per_thousand),
				);
				let whitespace = u64::from(
					example
						.whitespace_per_thousand
						.abs_diff(evidence.whitespace_per_thousand),
				);
				let mean = u64::try_from(example.mean_value_bytes.abs_diff(evidence.mean_value_bytes))
					.unwrap_or(u64::MAX / 16);
				let representation = u64::from(example.dictionary_smaller != dictionary_smaller);
				unique.saturating_mul(unique)
					.saturating_add(whitespace.saturating_mul(whitespace))
					.saturating_add(mean.saturating_mul(mean).saturating_mul(16))
					.saturating_add(representation.saturating_mul(250_000))
			})
			.map_or(SemanticType::Categorical, |example| example.label)
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InferredVector {
	index: usize,
	name: Vec<u8>,
	semantic_type: SemanticType,
	encoding: VectorEncoding,
	evidence: VectorEvidence,
}

impl InferredVector {
	#[must_use]
	pub const fn index(&self) -> usize {
		self.index
	}

	#[must_use]
	pub fn name(&self) -> &[u8] {
		&self.name
	}

	#[must_use]
	pub const fn semantic_type(&self) -> SemanticType {
		self.semantic_type
	}

	#[must_use]
	pub const fn encoding(&self) -> VectorEncoding {
		self.encoding
	}

	#[must_use]
	pub const fn evidence(&self) -> VectorEvidence {
		self.evidence
	}
}

/// A filesystem table object represented as its ordered list of feature
/// vectors. Rows remain in [`RawTable`]; this value classifies columns without
/// creating additional features.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InferredVectorList {
	vectors: Vec<InferredVector>,
}

impl InferredVectorList {
	#[must_use]
	pub fn vectors(&self) -> &[InferredVector] {
		&self.vectors
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct VectorSemantic {
	pub semantic_type: SemanticType,
	pub encoding: VectorEncoding,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum VectorSemanticRule {
	Infer,
	Classify,
	Exact(VectorSemantic),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SemanticErrorKind {
	InconsistentWidth,
	ArithmeticOverflow,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticError {
	pub kind: SemanticErrorKind,
	pub detail: String,
}

impl SemanticError {
	fn new(kind: SemanticErrorKind, detail: impl Into<String>) -> Self {
		Self {
			kind,
			detail: detail.into(),
		}
	}
}

impl fmt::Display for SemanticError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(formatter, "{:?}: {}", self.kind, self.detail)
	}
}

impl std::error::Error for SemanticError {}

pub type SemanticResult<T> = Result<T, SemanticError>;

/// Infer one and only one Recipe semantic type for every table column.
///
/// Parsers are tried before the ambiguous classifier: image signatures,
/// temporal syntax, ordinal vocabularies, exact int32, and exact f32. The
/// remaining vector is classified from its complete value list and smallest
/// lossless dictionary/UTF-8 representation.
pub fn infer_table_vectors(table: &RawTable, model: &impl AmbiguousVectorModel) -> SemanticResult<InferredVectorList> {
	infer_table_vectors_with_semantics(table, model, &[])
}

pub(crate) fn infer_table_vectors_with_semantics(
	table: &RawTable,
	model: &impl AmbiguousVectorModel,
	semantics: &[VectorSemanticRule],
) -> SemanticResult<InferredVectorList> {
	let width = table.width();
	for (row_index, row) in table.rows().iter().enumerate() {
		if row.len() != width {
			return Err(SemanticError::new(
				SemanticErrorKind::InconsistentWidth,
				format!(
					"row {row_index} contains {} values, expected {width}",
					row.len()
				),
			));
		}
	}
	let mut vectors = Vec::with_capacity(width);
	for index in 0..width {
		let values = table
			.rows()
			.iter()
			.map(|row| row[index].as_slice())
			.collect::<Vec<_>>();
		let evidence = collect_evidence(&values)?;
		let (semantic_type, encoding) = match semantics
			.get(index)
			.copied()
			.unwrap_or(VectorSemanticRule::Infer)
		{
			VectorSemanticRule::Infer => classify_vector(&values, evidence, model),
			VectorSemanticRule::Classify => {
				let semantic_type = model.classify(evidence);
				(semantic_type, encoding_for(semantic_type))
			}
			VectorSemanticRule::Exact(semantic) => (semantic.semantic_type, semantic.encoding),
		};
		let name = table
			.headers()
			.get(index)
			.cloned()
			.unwrap_or_else(|| format!("col{}", index + 1).into_bytes());
		vectors.push(InferredVector {
			index,
			name,
			semantic_type,
			encoding,
			evidence,
		});
	}
	Ok(InferredVectorList { vectors })
}

fn classify_vector(
	values: &[&[u8]],
	evidence: VectorEvidence,
	model: &impl AmbiguousVectorModel,
) -> (SemanticType, VectorEncoding) {
	let present = values.iter().copied().filter(|value| !value.is_empty());
	let present_values = present.clone().collect::<Vec<_>>();
	if !present_values.is_empty() && present.clone().all(is_image_value) {
		return (SemanticType::Image, VectorEncoding::Bytes);
	}
	if !present_values.is_empty() && present.clone().all(is_temporal_value) {
		return (SemanticType::Temporal, VectorEncoding::RelativeSecondsI32);
	}
	if is_ordinal_vector(&present_values) {
		return (SemanticType::Ordinal, VectorEncoding::OrdinalI32);
	}
	if !present_values.is_empty()
		&& present
			.clone()
			.all(|value| core::str::from_utf8(value).is_ok_and(|text| parse_contract_i32(text).is_ok()))
	{
		return (SemanticType::Numeric, VectorEncoding::I32);
	}
	if !present_values.is_empty()
		&& present
			.clone()
			.all(|value| core::str::from_utf8(value).is_ok_and(|text| parse_contract_f32(text).is_ok()))
	{
		return (SemanticType::Numeric, VectorEncoding::F32);
	}
	let semantic_type = model.classify(evidence);
	let encoding = encoding_for(semantic_type);
	(semantic_type, encoding)
}

const fn encoding_for(semantic_type: SemanticType) -> VectorEncoding {
	match semantic_type {
		SemanticType::Categorical => VectorEncoding::DictionaryI32,
		SemanticType::Text => VectorEncoding::Utf8,
		SemanticType::Numeric => VectorEncoding::F32,
		SemanticType::Temporal => VectorEncoding::RelativeSecondsI32,
		SemanticType::Ordinal => VectorEncoding::OrdinalI32,
		SemanticType::Image => VectorEncoding::Bytes,
		SemanticType::Binary => VectorEncoding::Bytes,
	}
}

fn collect_evidence(values: &[&[u8]]) -> SemanticResult<VectorEvidence> {
	let mut unique = BTreeSet::new();
	let mut missing = 0usize;
	let mut utf8_values = 0usize;
	let mut whitespace_values = 0usize;
	let mut source_bytes = 0usize;
	for value in values {
		source_bytes = source_bytes.checked_add(value.len()).ok_or_else(|| {
			SemanticError::new(
				SemanticErrorKind::ArithmeticOverflow,
				"aggregate vector byte count overflowed usize",
			)
		})?;
		if value.is_empty() {
			missing = missing.checked_add(1).ok_or_else(|| {
				SemanticError::new(
					SemanticErrorKind::ArithmeticOverflow,
					"missing value count overflowed usize",
				)
			})?;
			continue;
		}
		unique.insert(*value);
		if let Ok(text) = core::str::from_utf8(value) {
			utf8_values = utf8_values.checked_add(1).ok_or_else(|| {
				SemanticError::new(
					SemanticErrorKind::ArithmeticOverflow,
					"UTF-8 value count overflowed usize",
				)
			})?;
			if text.bytes().any(|byte| byte.is_ascii_whitespace()) {
				whitespace_values = whitespace_values.checked_add(1).ok_or_else(|| {
					SemanticError::new(
						SemanticErrorKind::ArithmeticOverflow,
						"whitespace value count overflowed usize",
					)
				})?;
			}
		}
	}
	let present = values.len().saturating_sub(missing);
	let unique_payload = unique.iter().try_fold(0usize, |total, value| {
		total.checked_add(value.len()).ok_or_else(|| {
			SemanticError::new(
				SemanticErrorKind::ArithmeticOverflow,
				"dictionary payload byte count overflowed usize",
			)
		})
	})?;
	let code_bytes = values
		.len()
		.checked_mul(core::mem::size_of::<i32>())
		.ok_or_else(|| {
			SemanticError::new(
				SemanticErrorKind::ArithmeticOverflow,
				"dictionary code byte count overflowed usize",
			)
		})?;
	let dictionary_bytes = unique_payload.checked_add(code_bytes).ok_or_else(|| {
		SemanticError::new(
			SemanticErrorKind::ArithmeticOverflow,
			"dictionary byte count overflowed usize",
		)
	})?;
	Ok(VectorEvidence {
		values: values.len(),
		missing,
		unique: unique.len(),
		utf8_values,
		whitespace_values,
		source_bytes,
		dictionary_bytes,
		mean_value_bytes: source_bytes.checked_div(present).unwrap_or(0),
		unique_per_thousand: ratio_per_thousand(unique.len(), present),
		whitespace_per_thousand: ratio_per_thousand(whitespace_values, present),
	})
}

fn ratio_per_thousand(numerator: usize, denominator: usize) -> u16 {
	if denominator == 0 {
		return 0;
	}
	let scaled = numerator.saturating_mul(1_000) / denominator;
	u16::try_from(scaled.min(1_000)).unwrap_or(1_000)
}

fn is_image_value(value: &[u8]) -> bool {
	has_recognized_image_signature(value)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct TemporalInstant {
	pub(crate) unix_seconds: i64,
	pub(crate) nanoseconds: u32,
}

pub(crate) fn parse_temporal_instant(value: &[u8]) -> Option<TemporalInstant> {
	let Ok(text) = core::str::from_utf8(value) else {
		return None;
	};
	let bytes = text.as_bytes();
	if bytes.len() < 10
		|| bytes.get(4) != Some(&b'-')
		|| bytes.get(7) != Some(&b'-')
		|| !valid_decimal_range(&bytes[0..4], 1, 9_999)
		|| !valid_decimal_range(&bytes[5..7], 1, 12)
	{
		return None;
	}
	let year = decimal_value(&bytes[0..4]).unwrap_or(0);
	let month = decimal_value(&bytes[5..7]).unwrap_or(0);
	let day_max = days_in_month(year, month);
	if !valid_decimal_range(&bytes[8..10], 1, day_max) {
		return None;
	}
	let day = decimal_value(&bytes[8..10])?;
	let mut hour = 0u32;
	let mut minute = 0u32;
	let mut second = 0u32;
	let mut nanoseconds = 0u32;
	let mut offset_seconds = 0i64;
	if bytes.len() == 10 {
		// Date-only values denote midnight UTC.
	} else {
		if !matches!(bytes.get(10), Some(b'T' | b' ')) || bytes.len() < 19 {
			return None;
		}
		if bytes.get(13) != Some(&b':')
			|| bytes.get(16) != Some(&b':')
			|| !valid_decimal_range(&bytes[11..13], 0, 23)
			|| !valid_decimal_range(&bytes[14..16], 0, 59)
			|| !valid_decimal_range(&bytes[17..19], 0, 60)
		{
			return None;
		}
		hour = decimal_value(&bytes[11..13])?;
		minute = decimal_value(&bytes[14..16])?;
		second = decimal_value(&bytes[17..19])?;
		let mut cursor = 19usize;
		if bytes.get(cursor) == Some(&b'.') {
			cursor += 1;
			let start = cursor;
			while bytes.get(cursor).is_some_and(u8::is_ascii_digit) {
				cursor += 1;
			}
			if cursor == start {
				return None;
			}
			let fraction = &bytes[start..cursor];
			let retained = fraction.len().min(9);
			nanoseconds = decimal_value(&fraction[..retained])?;
			for _ in retained..9 {
				nanoseconds = nanoseconds.checked_mul(10)?;
			}
			if fraction[retained..].iter().any(|digit| *digit != b'0') {
				return None;
			}
		}
		match bytes.get(cursor) {
			None => {}
			Some(b'Z') if cursor + 1 == bytes.len() => {}
			Some(sign @ (b'+' | b'-'))
				if cursor + 6 == bytes.len()
					&& bytes.get(cursor + 3) == Some(&b':')
					&& valid_decimal_range(&bytes[cursor + 1..cursor + 3], 0, 23)
					&& valid_decimal_range(&bytes[cursor + 4..cursor + 6], 0, 59) =>
			{
				let offset_hour = i64::from(decimal_value(&bytes[cursor + 1..cursor + 3])?);
				let offset_minute = i64::from(decimal_value(&bytes[cursor + 4..cursor + 6])?);
				let magnitude = offset_hour
					.checked_mul(3_600)?
					.checked_add(offset_minute.checked_mul(60)?)?;
				offset_seconds = if *sign == b'+' { magnitude } else { -magnitude };
			}
			Some(_) => return None,
		}
	}
	let days = days_from_civil(i64::from(year), i64::from(month), i64::from(day));
	let seconds = days
		.checked_mul(86_400)?
		.checked_add(i64::from(hour).checked_mul(3_600)?)?
		.checked_add(i64::from(minute).checked_mul(60)?)?
		.checked_add(i64::from(second))?
		.checked_sub(offset_seconds)?;
	Some(TemporalInstant {
		unix_seconds: seconds,
		nanoseconds,
	})
}

fn is_temporal_value(value: &[u8]) -> bool {
	parse_temporal_instant(value).is_some()
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
	let adjusted_year = year - i64::from(month <= 2);
	let era = adjusted_year.div_euclid(400);
	let year_of_era = adjusted_year - era * 400;
	let adjusted_month = month + if month > 2 { -3 } else { 9 };
	let day_of_year = (153 * adjusted_month + 2) / 5 + day - 1;
	let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
	era * 146_097 + day_of_era - 719_468
}

fn decimal_value(bytes: &[u8]) -> Option<u32> {
	bytes.iter().try_fold(0u32, |value, byte| {
		byte.is_ascii_digit().then(|| {
			value.saturating_mul(10)
				.saturating_add(u32::from(byte - b'0'))
		})
	})
}

fn valid_decimal_range(bytes: &[u8], minimum: u32, maximum: u32) -> bool {
	decimal_value(bytes).is_some_and(|value| (minimum..=maximum).contains(&value))
}

const fn days_in_month(year: u32, month: u32) -> u32 {
	match month {
		1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
		4 | 6 | 9 | 11 => 30,
		2 if year.is_multiple_of(400) || year.is_multiple_of(4) && !year.is_multiple_of(100) => 29,
		2 => 28,
		_ => 0,
	}
}

const ORDINAL_VOCABULARIES: [&[&[u8]]; 6] = [
	&[b"low", b"medium", b"high"],
	&[b"small", b"medium", b"large"],
	&[b"beginner", b"intermediate", b"advanced"],
	&[b"poor", b"fair", b"good", b"very good", b"excellent"],
	&[b"first", b"second", b"third", b"fourth", b"fifth"],
	&[b"bronze", b"silver", b"gold", b"platinum"],
];

pub(crate) fn ordinal_vocabulary(values: &[&[u8]]) -> Option<&'static [&'static [u8]]> {
	if values.len() < 2 {
		return None;
	}
	ORDINAL_VOCABULARIES.iter().copied().find(|order| {
		let mut seen = BTreeSet::new();
		values.iter().all(|value| {
			order.iter()
				.position(|candidate| value.eq_ignore_ascii_case(candidate))
				.is_some_and(|position| seen.insert(position) || seen.contains(&position))
		}) && seen.len() >= 2
	})
}

/// Resolve one ordinal vocabulary from fit-partition values only.
///
/// A single fit label is sufficient when it identifies exactly one declared
/// order. Ambiguous or empty fit evidence deliberately chooses no vocabulary;
/// validation labels never participate in disambiguation.
pub(crate) fn fit_ordinal_vocabulary(values: &[&[u8]]) -> Option<&'static [&'static [u8]]> {
	if values.is_empty() {
		return None;
	}
	let mut candidates = ORDINAL_VOCABULARIES.iter().copied().filter(|order| {
		values.iter().all(|value| {
			order.iter()
				.any(|candidate| value.eq_ignore_ascii_case(candidate))
		})
	});
	let vocabulary = candidates.next()?;
	(!candidates.any(|_| true)).then_some(vocabulary)
}

fn is_ordinal_vector(values: &[&[u8]]) -> bool {
	ordinal_vocabulary(values).is_some()
}

#[cfg(test)]
mod tests {
	use crate::{Delimiter, HeaderMode, IngestLimits, TableRequest, parse_table};

	use super::*;

	fn table(bytes: &[u8]) -> RawTable {
		parse_table(
			bytes,
			TableRequest::new(
				Delimiter::Comma,
				HeaderMode::Present,
				IngestLimits::new(16_384, 128, 32, 1_024).unwrap(),
			),
		)
		.unwrap()
	}

	#[test]
	fn infers_all_nonbinary_table_semantic_types() {
		let raw = table(b"number,time,category,rank,prose,picture\n\
			1,2024-02-29T12:00:00Z,red,low,\"a sentence with several spaces\",\xff\xd8\xffone\n\
			2,2024-03-01T12:00:00Z,blue,medium,\"another sentence with several spaces\",\xff\xd8\xfftwo\n\
			3,2024-03-02T12:00:00Z,red,high,\"a third sentence with several spaces\",\xff\xd8\xffthree\n");
		assert_eq!(raw.width(), 6);
		let inferred = infer_table_vectors(&raw, &CategoricalEncodingModel).unwrap();
		assert_eq!(
			inferred
				.vectors()
				.iter()
				.map(InferredVector::semantic_type)
				.collect::<Vec<_>>(),
			[
				SemanticType::Numeric,
				SemanticType::Temporal,
				SemanticType::Categorical,
				SemanticType::Ordinal,
				SemanticType::Text,
				SemanticType::Image,
			]
		);
	}

	#[test]
	fn recognizes_ordinals_only_when_the_vector_establishes_an_order_vocabulary() {
		let raw = table(b"priority\nlow\nmedium\nhigh\n");
		let inferred = infer_table_vectors(&raw, &CategoricalEncodingModel).unwrap();
		assert_eq!(inferred.vectors()[0].semantic_type(), SemanticType::Ordinal);
		assert_eq!(inferred.vectors()[0].encoding(), VectorEncoding::OrdinalI32);
	}

	#[test]
	fn temporal_parser_validates_calendar_and_zone_syntax() {
		assert!(is_temporal_value(b"2024-02-29"));
		assert!(is_temporal_value(b"2016-04-29T18:38:08Z"));
		assert!(is_temporal_value(b"2025-01-31 23:59:60-08:00"));
		assert!(!is_temporal_value(b"2023-02-29"));
		assert!(!is_temporal_value(b"2024-13-01"));
		assert!(!is_temporal_value(b"2024-01-01T25:00:00Z"));
	}

	#[test]
	fn exact_payload_limits_choose_i32_then_f32() {
		let integers = table(b"x\n999999999\n-12\n");
		let inferred = infer_table_vectors(&integers, &CategoricalEncodingModel).unwrap();
		assert_eq!(inferred.vectors()[0].encoding(), VectorEncoding::I32);

		let floats = table(b"x\n1.23456\n-0.125\n");
		let inferred = infer_table_vectors(&floats, &CategoricalEncodingModel).unwrap();
		assert_eq!(inferred.vectors()[0].encoding(), VectorEncoding::F32);
	}

	#[test]
	fn ambiguity_model_uses_complete_vector_storage_and_text_shape() {
		let categories = table(b"x\nalpha\nbeta\nalpha\nbeta\n");
		let prose = table(b"x\n\"this is a complete sentence with several words\"\n\
			\"another distinct sentence containing several words\"\n\
			\"a third distinct sentence containing several words\"\n");
		assert_eq!(
			infer_table_vectors(&categories, &CategoricalEncodingModel)
				.unwrap()
				.vectors()[0]
				.semantic_type(),
			SemanticType::Categorical
		);
		assert_eq!(
			infer_table_vectors(&prose, &CategoricalEncodingModel)
				.unwrap()
				.vectors()[0]
				.semantic_type(),
			SemanticType::Text
		);
	}
}
