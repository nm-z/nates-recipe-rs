use core::{cmp::Ordering, fmt, num::NonZeroU64};
use std::collections::{BTreeMap, BTreeSet};

use crate::{
	AmbiguousVectorModel, InferredVector, InferredVectorList, RawTable, SemanticError, SemanticType, VectorEncoding,
	image_header::{EncodedImageMetadata, inspect_encoded_image},
	parse_contract_f32, parse_contract_i32,
	semantic::{
		TemporalInstant, VectorSemantic, VectorSemanticRule, fit_ordinal_vocabulary,
		infer_table_vectors_with_semantics, parse_temporal_instant,
	},
};

/// An exact rational train share. No randomization or host floating-point
/// rounding participates in partition construction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrainFraction {
	numerator: u64,
	denominator: NonZeroU64,
}

impl TrainFraction {
	/// Construct a fraction strictly between zero and one.
	///
	/// # Errors
	///
	/// Returns [`PrepareErrorKind::InvalidTrainFraction`] unless
	/// `0 < numerator < denominator`.
	pub fn new(numerator: u64, denominator: u64) -> PrepareResult<Self> {
		let denominator = NonZeroU64::new(denominator).ok_or_else(|| {
			PrepareError::new(
				PrepareErrorKind::InvalidTrainFraction,
				"train fraction denominator must be nonzero",
			)
		})?;
		if numerator == 0 || numerator >= denominator.get() {
			return Err(PrepareError::new(
				PrepareErrorKind::InvalidTrainFraction,
				format!(
					"train fraction must be strictly between zero and one, got {numerator}/{}",
					denominator.get()
				),
			));
		}
		let divisor = greatest_common_divisor(numerator, denominator.get());
		Ok(Self {
			numerator: numerator / divisor,
			denominator: NonZeroU64::new(denominator.get() / divisor).unwrap_or(NonZeroU64::MIN),
		})
	}

	/// Preserve the exact binary value of a finite f32 split declaration.
	///
	/// # Errors
	///
	/// Returns [`PrepareErrorKind::InvalidTrainFraction`] when the value is
	/// outside `(0, 1)` or its exact denominator exceeds this API's u64
	/// representation.
	pub fn from_f32(value: f32) -> PrepareResult<Self> {
		if !value.is_finite() || !(0.0..1.0).contains(&value) {
			return Err(PrepareError::new(
				PrepareErrorKind::InvalidTrainFraction,
				format!("f32 train fraction must be finite and in (0, 1), got {value}"),
			));
		}
		let bits = value.to_bits();
		let exponent = (bits >> 23) & 0xff;
		let fraction = bits & 0x7f_ffff;
		let (mut numerator, binary_exponent) = if exponent == 0 {
			(u64::from(fraction), -149i32)
		} else {
			(
				u64::from((1 << 23) | fraction),
				i32::try_from(exponent).unwrap_or(0) - 150,
			)
		};
		let denominator_power = binary_exponent.checked_neg().ok_or_else(|| {
			PrepareError::new(
				PrepareErrorKind::InvalidTrainFraction,
				"f32 train fraction exponent cannot be represented",
			)
		})?;
		let removable = numerator
			.trailing_zeros()
			.min(u32::try_from(denominator_power).unwrap_or(u32::MAX));
		numerator >>= removable;
		let reduced_power = denominator_power - i32::try_from(removable).unwrap_or(0);
		if reduced_power >= 64 {
			return Err(PrepareError::new(
				PrepareErrorKind::InvalidTrainFraction,
				"exact f32 train fraction denominator exceeds u64",
			));
		}
		let denominator = 1u64
			.checked_shl(u32::try_from(reduced_power).unwrap_or(u32::MAX))
			.ok_or_else(|| {
				PrepareError::new(
					PrepareErrorKind::InvalidTrainFraction,
					"exact f32 train fraction denominator exceeds u64",
				)
			})?;
		Self::new(numerator, denominator)
	}

	#[must_use]
	pub const fn numerator(self) -> u64 { self.numerator }

	#[must_use]
	pub const fn denominator(self) -> NonZeroU64 { self.denominator }

	fn train_rows(self, retained_rows: usize) -> PrepareResult<usize> {
		let rows = u128::try_from(retained_rows).map_err(|error| {
			PrepareError::new(
				PrepareErrorKind::ArithmeticOverflow,
				format!("retained row count cannot be represented as u128: {error}"),
			)
		})?;
		let product = rows
			.checked_mul(u128::from(self.numerator))
			.ok_or_else(|| {
				PrepareError::new(
					PrepareErrorKind::ArithmeticOverflow,
					"train partition row calculation overflowed u128",
				)
			})?;
		let count = product / u128::from(self.denominator.get());
		usize::try_from(count).map_err(|error| {
			PrepareError::new(
				PrepareErrorKind::ArithmeticOverflow,
				format!("train partition row count cannot be represented as usize: {error}"),
			)
		})
	}
}

impl TryFrom<f32> for TrainFraction {
	type Error = PrepareError;

	fn try_from(value: f32) -> Result<Self, Self::Error> { Self::from_f32(value) }
}

const fn greatest_common_divisor(mut left: u64, mut right: u64) -> u64 {
	while right != 0 {
		let remainder = left % right;
		left = right;
		right = remainder;
	}
	left
}

/// A case-insensitive byte glob for source column names.
///
/// `*` matches zero or more bytes and `?` matches one byte. All other bytes
/// are literal. ASCII case is folded so `*ID*` also matches `PatientId`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColumnPattern {
	pattern: Vec<u8>,
}

impl ColumnPattern {
	/// Create a nonempty exclusion pattern.
	///
	/// # Errors
	///
	/// Returns [`PrepareErrorKind::InvalidColumnPattern`] for an empty pattern.
	pub fn new(pattern: impl Into<Vec<u8>>) -> PrepareResult<Self> {
		let pattern = pattern.into();
		if pattern.is_empty() {
			return Err(PrepareError::new(
				PrepareErrorKind::InvalidColumnPattern,
				"column exclusion pattern must not be empty",
			));
		}
		Ok(Self { pattern })
	}

	#[must_use]
	pub fn as_bytes(&self) -> &[u8] { &self.pattern }

	fn matches(&self, name: &[u8]) -> bool { glob_matches(&self.pattern, name) }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComparisonOperator {
	Equal,
	NotEqual,
	Less,
	LessOrEqual,
	Greater,
	GreaterOrEqual,
}

/// A typed constant used to exclude matching source rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PredicateLiteral {
	Signed(i64),
	Unsigned(u64),
	/// Exact IEEE-754 bits. NaN and infinity fail during request validation.
	F32Bits(u32),
	Text(String),
}

impl PredicateLiteral {
	#[must_use]
	pub const fn f32(value: f32) -> Self { Self::F32Bits(value.to_bits()) }
}

/// A source-row exclusion. A row is removed when this predicate evaluates
/// true. Multiple predicates are combined with logical OR.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RowPredicate {
	column: Vec<u8>,
	operator: ComparisonOperator,
	literal: PredicateLiteral,
}

impl RowPredicate {
	#[must_use]
	pub fn new(column: impl Into<Vec<u8>>, operator: ComparisonOperator, literal: PredicateLiteral) -> Self {
		Self {
			column: column.into(),
			operator,
			literal,
		}
	}

	#[must_use]
	pub fn column(&self) -> &[u8] { &self.column }

	#[must_use]
	pub const fn operator(&self) -> ComparisonOperator { self.operator }

	#[must_use]
	pub const fn literal(&self) -> &PredicateLiteral { &self.literal }
}

/// Complete, declarative table preparation policy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparationRequest {
	targets: Vec<Vec<u8>>,
	excluded_columns: Vec<ColumnPattern>,
	excluded_rows: Vec<RowPredicate>,
	train_fraction: TrainFraction,
}

impl PreparationRequest {
	#[must_use]
	pub fn new<I, S>(targets: I, train_fraction: TrainFraction) -> Self
	where
		I: IntoIterator<Item = S>,
		S: Into<Vec<u8>>,
	{
		Self {
			targets: targets.into_iter().map(Into::into).collect(),
			excluded_columns: Vec::new(),
			excluded_rows: Vec::new(),
			train_fraction,
		}
	}

	#[must_use]
	pub fn exclude_columns(mut self, patterns: impl IntoIterator<Item = ColumnPattern>) -> Self {
		self.excluded_columns.extend(patterns);
		self
	}

	#[must_use]
	pub fn exclude_rows(mut self, predicates: impl IntoIterator<Item = RowPredicate>) -> Self {
		self.excluded_rows.extend(predicates);
		self
	}

	#[must_use]
	pub fn targets(&self) -> &[Vec<u8>] { &self.targets }

	#[must_use]
	pub fn excluded_columns(&self) -> &[ColumnPattern] { &self.excluded_columns }

	#[must_use]
	pub fn excluded_rows(&self) -> &[RowPredicate] { &self.excluded_rows }

	#[must_use]
	pub const fn train_fraction(&self) -> TrainFraction { self.train_fraction }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VectorRole {
	Feature,
	Target,
}

/// UTC origin retained alongside a relative-second temporal vector.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TemporalOrigin {
	pub unix_seconds: i64,
	pub nanoseconds: u32,
}

/// Encoding-specific metadata required to invert semantic values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VectorMetadata {
	None,
	Temporal {
		origin: TemporalOrigin,
	},
	Categorical {
		/// Known code `n` denotes `dictionary[n]`. Code `dictionary.len()` is
		/// the calculation-facing route for a nonempty value not observed in
		/// the fit partition. [`CategoricalObservation`] retains whether every
		/// row was known, missing, or unseen and preserves unseen label bytes.
		dictionary: Vec<Vec<u8>>,
	},
	Ordinal {
		/// Rank `n` denotes `ordered_labels[n]`.
		ordered_labels: Vec<Vec<u8>>,
	},
	Image {
		/// Exact, deterministic set of encoded headers observed in retained
		/// nonmissing values. Multiple entries preserve mixed formats or shapes.
		encoded_variants: Vec<EncodedImageMetadata>,
	},
}

impl VectorMetadata {
	/// Return whether this vector has no encoding-specific metadata.
	#[must_use]
	#[inline]
	pub const fn is_none(&self) -> bool {
		matches!(self, Self::None)
	}

	/// Borrow the temporal origin when this is temporal metadata.
	#[must_use]
	#[inline]
	pub const fn temporal_origin(&self) -> Option<TemporalOrigin> {
		match self {
			Self::Temporal { origin } => Some(*origin),
			Self::None | Self::Categorical { .. } | Self::Ordinal { .. } | Self::Image { .. } => None,
		}
	}

	/// Borrow the fitted dictionary when this is categorical metadata.
	#[must_use]
	#[inline]
	pub fn categorical_dictionary(&self) -> Option<&[Vec<u8>]> {
		match self {
			Self::Categorical { dictionary } => Some(dictionary),
			Self::None | Self::Temporal { .. } | Self::Ordinal { .. } | Self::Image { .. } => None,
		}
	}

	/// Borrow the fitted label order when this is ordinal metadata.
	#[must_use]
	#[inline]
	pub fn ordinal_labels(&self) -> Option<&[Vec<u8>]> {
		match self {
			Self::Ordinal { ordered_labels } => Some(ordered_labels),
			Self::None | Self::Temporal { .. } | Self::Categorical { .. } | Self::Image { .. } => None,
		}
	}

	/// Return whether this is encoded-image metadata.
	#[must_use]
	#[inline]
	pub const fn is_image(&self) -> bool {
		matches!(self, Self::Image { .. })
	}
}

/// The lossless categorical observation route recorded for one retained row.
///
/// Calculation codes remain available in [`PreparedValues::I32`] for existing
/// graph consumers. This typed route is authoritative for observation identity:
/// missing input is not an unseen label, and an unseen nonempty label retains
/// its exact source bytes instead of being represented only by a reserved code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CategoricalObservation {
	Known { code: i32 },
	Missing,
	Unseen { label: Vec<u8> },
}

impl CategoricalObservation {
	#[must_use]
	pub const fn known_code(&self) -> Option<i32> {
		match self {
			Self::Known { code } => Some(*code),
			Self::Missing | Self::Unseen { .. } => None,
		}
	}

	#[must_use]
	pub fn unseen_label(&self) -> Option<&[u8]> {
		match self {
			Self::Unseen { label } => Some(label),
			Self::Known { .. } | Self::Missing => None,
		}
	}
}

/// Packed values for exactly one retained source vector.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PreparedValues {
	I32(Vec<Option<i32>>),
	/// Exact IEEE-754 bit patterns avoid accidental host-side arithmetic.
	F32Bits(Vec<Option<u32>>),
	VariableWidth(VariableWidthVector),
}

impl PreparedValues {
	/// Borrow dictionary or integer values when this vector uses `I32` storage.
	#[must_use]
	#[inline]
	pub fn i32_values(&self) -> Option<&[Option<i32>]> {
		match self {
			Self::I32(values) => Some(values),
			Self::F32Bits(_) | Self::VariableWidth(_) => None,
		}
	}

	/// Borrow exact f32 bits when this vector uses `F32Bits` storage.
	#[must_use]
	#[inline]
	pub fn f32_bits(&self) -> Option<&[Option<u32>]> {
		match self {
			Self::F32Bits(values) => Some(values),
			Self::I32(_) | Self::VariableWidth(_) => None,
		}
	}

	/// Borrow variable-width values when this vector uses variable-width storage.
	#[must_use]
	#[inline]
	pub const fn variable_width(&self) -> Option<&VariableWidthVector> {
		match self {
			Self::VariableWidth(values) => Some(values),
			Self::I32(_) | Self::F32Bits(_) => None,
		}
	}

	#[must_use]
	pub fn len(&self) -> usize {
		match self {
			Self::I32(values) => values.len(),
			Self::F32Bits(values) => values.len(),
			Self::VariableWidth(values) => values.len(),
		}
	}

	#[must_use]
	pub fn is_empty(&self) -> bool { self.len() == 0 }
}

/// Lossless offset/payload representation for text and image vectors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VariableWidthVector {
	offsets: Vec<u64>,
	payload: Vec<u8>,
	valid: Vec<bool>,
}

impl VariableWidthVector {
	#[must_use]
	pub fn len(&self) -> usize { self.valid.len() }

	#[must_use]
	pub fn is_empty(&self) -> bool { self.valid.is_empty() }

	#[must_use]
	pub fn offsets(&self) -> &[u64] { &self.offsets }

	#[must_use]
	pub fn payload(&self) -> &[u8] { &self.payload }

	#[must_use]
	pub fn validity(&self) -> &[bool] { &self.valid }

	#[must_use]
	pub fn value(&self, retained_row: usize) -> Option<Option<&[u8]>> {
		let valid = *self.valid.get(retained_row)?;
		let start = usize::try_from(*self.offsets.get(retained_row)?).ok()?;
		let end = usize::try_from(*self.offsets.get(retained_row.checked_add(1)?)?).ok()?;
		Some(valid.then(|| &self.payload[start..end]))
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedVector {
	source_index: usize,
	name: Vec<u8>,
	role: VectorRole,
	semantic_type: SemanticType,
	encoding: VectorEncoding,
	metadata: VectorMetadata,
	values: PreparedValues,
	categorical_observations: Option<Vec<CategoricalObservation>>,
}

/// Row-free semantic identity of one prepared source vector.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VectorSchema {
	source_index: usize,
	name: Vec<u8>,
	role: VectorRole,
	semantic_type: SemanticType,
	encoding: VectorEncoding,
	metadata: VectorMetadata,
}

impl VectorSchema {
	#[must_use]
	pub const fn source_index(&self) -> usize { self.source_index }

	#[must_use]
	pub fn name(&self) -> &[u8] { &self.name }

	#[must_use]
	pub const fn role(&self) -> VectorRole { self.role }

	#[must_use]
	pub const fn semantic_type(&self) -> SemanticType { self.semantic_type }

	#[must_use]
	pub const fn encoding(&self) -> VectorEncoding { self.encoding }

	#[must_use]
	pub const fn metadata(&self) -> &VectorMetadata { &self.metadata }
}

impl PreparedVector {
	#[must_use]
	pub fn schema(&self) -> VectorSchema {
		VectorSchema {
			source_index: self.source_index,
			name: self.name.clone(),
			role: self.role,
			semantic_type: self.semantic_type,
			encoding: self.encoding,
			metadata: self.metadata.clone(),
		}
	}

	#[must_use]
	pub const fn source_index(&self) -> usize { self.source_index }

	#[must_use]
	pub fn name(&self) -> &[u8] { &self.name }

	#[must_use]
	pub const fn role(&self) -> VectorRole { self.role }

	#[must_use]
	pub const fn semantic_type(&self) -> SemanticType { self.semantic_type }

	#[must_use]
	pub const fn encoding(&self) -> VectorEncoding { self.encoding }

	#[must_use]
	pub const fn metadata(&self) -> &VectorMetadata { &self.metadata }

	#[must_use]
	pub const fn values(&self) -> &PreparedValues { &self.values }

	/// Typed categorical observations in retained-row order.
	///
	/// This is `Some` exactly for dictionary-encoded categorical vectors. Its
	/// length and order are identical to [`Self::values`].
	#[must_use]
	pub fn categorical_observations(&self) -> Option<&[CategoricalObservation]> {
		self.categorical_observations.as_deref()
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartitionKind {
	Train,
	Validation,
}

/// Stable positions into every prepared vector plus their original source-row
/// indices.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedPartition {
	kind: PartitionKind,
	retained_positions: Vec<usize>,
	source_rows: Vec<usize>,
}

impl PreparedPartition {
	#[must_use]
	pub const fn kind(&self) -> PartitionKind { self.kind }

	#[must_use]
	pub fn retained_positions(&self) -> &[usize] { &self.retained_positions }

	#[must_use]
	pub fn source_rows(&self) -> &[usize] { &self.source_rows }

	#[must_use]
	pub fn len(&self) -> usize { self.retained_positions.len() }

	#[must_use]
	pub fn is_empty(&self) -> bool { self.retained_positions.is_empty() }
}

/// Homogeneous, row-major projection requested explicitly from prepared
/// vectors. Mixed fixed-width numeric vectors use f32 only for exact integer
/// conversions; missing values, text, images, and lossy conversions fail.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DenseMatrix {
	I32 {
		rows: usize,
		columns: usize,
		values: Vec<i32>,
	},
	F32Bits {
		rows: usize,
		columns: usize,
		values: Vec<u32>,
	},
}

impl DenseMatrix {
	#[must_use]
	pub const fn dtype(&self) -> recipe_core::DType {
		match self {
			Self::I32 { .. } => recipe_core::DType::I32,
			Self::F32Bits { .. } => recipe_core::DType::F32,
		}
	}

	#[must_use]
	pub const fn rows(&self) -> usize {
		match self {
			Self::I32 { rows, .. } | Self::F32Bits { rows, .. } => *rows,
		}
	}

	#[must_use]
	pub const fn columns(&self) -> usize {
		match self {
			Self::I32 { columns, .. } | Self::F32Bits { columns, .. } => *columns,
		}
	}
}

/// Preparation output. Vectors remain in source order and values remain
/// aligned to `retained_source_rows`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedDataset {
	source_row_count: usize,
	retained_source_rows: Vec<usize>,
	excluded_source_rows: Vec<usize>,
	vectors: Vec<PreparedVector>,
	target_source_indices: Vec<usize>,
	train: PreparedPartition,
	validation: PreparedPartition,
}

impl PreparedDataset {
	#[must_use]
	pub const fn source_row_count(&self) -> usize { self.source_row_count }

	#[must_use]
	pub fn retained_source_rows(&self) -> &[usize] { &self.retained_source_rows }

	#[must_use]
	pub fn excluded_source_rows(&self) -> &[usize] { &self.excluded_source_rows }

	#[must_use]
	pub fn vectors(&self) -> &[PreparedVector] { &self.vectors }

	/// Target-vector source identities in the user's declaration order.
	///
	/// Prepared vectors retain source-column order. This separate ordering is
	/// authoritative whenever one model produces one result per declared
	/// target, so source layout cannot silently reorder model outputs.
	#[must_use]
	pub fn target_source_indices(&self) -> &[usize] { &self.target_source_indices }

	#[must_use]
	pub const fn train(&self) -> &PreparedPartition { &self.train }

	#[must_use]
	pub const fn validation(&self) -> &PreparedPartition { &self.validation }

	/// Materialize one role and partition as a homogeneous row-major matrix.
	/// No normalization, imputation, lossy casting, or feature derivation
	/// occurs.
	///
	/// # Errors
	///
	/// Mixed i32/f32 columns produce f32 only when every integer is exactly
	/// representable. Variable-width vectors, lossy conversions, missing
	/// values, an absent role, and inconsistent internal lengths fail closed.
	pub fn fixed_dense_matrix(&self, role: VectorRole, partition: PartitionKind) -> PrepareResult<DenseMatrix> {
		let vectors = self
			.vectors
			.iter()
			.filter(|vector| vector.role == role)
			.collect::<Vec<_>>();
		if vectors.is_empty() {
			return Err(PrepareError::new(
				PrepareErrorKind::EmptyDenseSelection,
				format!("no {role:?} vectors are available"),
			));
		}
		let partition = match partition {
			PartitionKind::Train => &self.train,
			PartitionKind::Validation => &self.validation,
		};
		if let Some(vector) = vectors
			.iter()
			.find(|vector| matches!(vector.values, PreparedValues::VariableWidth(_)))
		{
			return Err(variable_dense_error(vector));
		}
		match vectors
			.iter()
			.all(|vector| matches!(vector.values, PreparedValues::I32(_)))
		{
			true => self.dense_i32(&vectors, partition),
			false => self.dense_f32(&vectors, partition),
		}
	}

	fn dense_i32(&self, vectors: &[&PreparedVector], partition: &PreparedPartition) -> PrepareResult<DenseMatrix> {
		let capacity = matrix_capacity(partition.len(), vectors.len())?;
		let mut output = Vec::with_capacity(capacity);
		for position in &partition.retained_positions {
			for vector in vectors {
				let PreparedValues::I32(values) = &vector.values else {
					return match vector.values {
						PreparedValues::VariableWidth(_) => Err(variable_dense_error(vector)),
						PreparedValues::F32Bits(_) => {
							unreachable!("dense_i32 is selected only for all-i32 vectors")
						}
						PreparedValues::I32(_) => unreachable!(),
					};
				};
				let value = values
					.get(*position)
					.ok_or_else(|| inconsistent_vector_error(vector, *position))?
					.ok_or_else(|| missing_dense_error(vector, self.source_row_at(*position)))?;
				output.push(value);
			}
		}
		Ok(DenseMatrix::I32 {
			rows: partition.len(),
			columns: vectors.len(),
			values: output,
		})
	}

	fn dense_f32(&self, vectors: &[&PreparedVector], partition: &PreparedPartition) -> PrepareResult<DenseMatrix> {
		let capacity = matrix_capacity(partition.len(), vectors.len())?;
		let mut output = Vec::with_capacity(capacity);
		for position in &partition.retained_positions {
			for vector in vectors {
				let value = match &vector.values {
					PreparedValues::F32Bits(values) => {
						values.get(*position)
							.ok_or_else(|| inconsistent_vector_error(vector, *position))?
							.ok_or_else(|| missing_dense_error(vector, self.source_row_at(*position)))?
					}
					PreparedValues::I32(values) => {
						let value = values
							.get(*position)
							.ok_or_else(|| inconsistent_vector_error(vector, *position))?
							.ok_or_else(|| missing_dense_error(vector, self.source_row_at(*position)))?;
						let converted = value as f32;
						if f64::from(converted) != f64::from(value) {
							return Err(mixed_dense_error(
								vector,
								self.source_row_at(*position),
								value,
							));
						}
						converted.to_bits()
					}
					PreparedValues::VariableWidth(_) => return Err(variable_dense_error(vector)),
				};
				output.push(value);
			}
		}
		Ok(DenseMatrix::F32Bits {
			rows: partition.len(),
			columns: vectors.len(),
			values: output,
		})
	}

	fn source_row_at(&self, retained_position: usize) -> Option<usize> {
		self.retained_source_rows.get(retained_position).copied()
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PrepareErrorKind {
	SemanticInference,
	InvalidTrainFraction,
	InvalidColumnPattern,
	EmptyTargetSet,
	DuplicateTarget,
	DuplicateColumnName,
	TargetNotFound,
	UnmatchedColumnPattern,
	TargetExcluded,
	NoFeatureVectors,
	NoRetainedRows,
	PredicateColumnNotFound,
	InvalidPredicateLiteral,
	PredicateTypeMismatch,
	MissingPredicateValue,
	InvalidPredicateValue,
	InconsistentInference,
	EncodingFailure,
	TemporalRangeExceeded,
	VariableWidthDenseMatrix,
	MixedDenseEncoding,
	MissingDenseValue,
	EmptyDenseSelection,
	InconsistentPreparedVector,
	ArithmeticOverflow,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrepareError {
	pub kind: PrepareErrorKind,
	pub column: Option<Vec<u8>>,
	pub source_row: Option<usize>,
	pub detail: String,
}

impl PrepareError {
	fn new(kind: PrepareErrorKind, detail: impl Into<String>) -> Self {
		Self {
			kind,
			column: None,
			source_row: None,
			detail: detail.into(),
		}
	}

	fn for_column(mut self, column: &[u8]) -> Self {
		self.column = Some(column.to_vec());
		self
	}

	fn for_row(mut self, source_row: usize) -> Self {
		self.source_row = Some(source_row);
		self
	}
}

impl fmt::Display for PrepareError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(formatter, "{:?}: {}", self.kind, self.detail)?;
		if let Some(column) = &self.column {
			write!(formatter, " [column {:?}]", String::from_utf8_lossy(column))?;
		}
		if let Some(source_row) = self.source_row {
			write!(formatter, " [source row {source_row}]")?;
		}
		Ok(())
	}
}

impl std::error::Error for PrepareError {}

pub type PrepareResult<T> = Result<T, PrepareError>;

/// Discover source semantics, fit preprocessing state only on the exact train
/// partition, then apply that immutable state to every retained row.
///
/// # Errors
///
/// Fails closed on ambiguous names, invalid predicates, unrepresentable
/// encodings, or semantic inference errors.
pub fn prepare_table(
	table: &RawTable,
	request: &PreparationRequest,
	model: &impl AmbiguousVectorModel,
) -> PrepareResult<PreparedDataset> {
	prepare_table_with_semantics(table, request, model, &[])
}

pub(crate) fn prepare_table_with_semantics(
	table: &RawTable,
	request: &PreparationRequest,
	model: &impl AmbiguousVectorModel,
	semantics: &[VectorSemanticRule],
) -> PrepareResult<PreparedDataset> {
	let (target_indices, excluded_columns, retained_source_rows, excluded_source_rows) =
		select_rows_and_columns_before_fit(table, request)?;
	let train_rows = request
		.train_fraction
		.train_rows(retained_source_rows.len())?;
	let fit_table = fit_partition_table(table, &retained_source_rows[..train_rows])?;
	let fit_semantics = fit_semantics_with_predicate_constraints(&fit_table, request, semantics)?;
	let fitted = infer_table_vectors_with_semantics(&fit_table, model, &fit_semantics).map_err(semantic_error)?;
	validate_inference(table, &fitted)?;
	validate_predicates_after_fit(request, &fitted)?;
	prepare_preselected_table(
		table,
		&fitted,
		&target_indices,
		&excluded_columns,
		retained_source_rows,
		excluded_source_rows,
		train_rows,
	)
}

/// Prepare a table using a caller-supplied authoritative semantic contract.
///
/// Exactly one output vector is emitted for each source vector retained after
/// column exclusion. Rows are excluded before the exact rational split.
/// Metadata is still fitted on train rows, but the supplied semantic types and
/// encodings are not re-inferred. Automatic callers must use [`prepare_table`]
/// or [`crate::DistilledDataset::prepare`] so validation rows cannot influence
/// semantic discovery.
///
/// # Errors
///
/// Fails closed when inference does not describe this table or when selection,
/// predicates, or lossless encoding cannot be completed.
pub fn prepare_inferred_table(
	table: &RawTable,
	inferred: &InferredVectorList,
	request: &PreparationRequest,
) -> PrepareResult<PreparedDataset> {
	validate_inference(table, inferred)?;
	let (target_indices, excluded_columns, retained_source_rows, excluded_source_rows) =
		select_rows_and_columns(table, inferred, request)?;
	let train_rows = request
		.train_fraction
		.train_rows(retained_source_rows.len())?;
	prepare_preselected_table(
		table,
		inferred,
		&target_indices,
		&excluded_columns,
		retained_source_rows,
		excluded_source_rows,
		train_rows,
	)
}

/// Apply column and row exclusions without declaring targets, fitting
/// semantics, or partitioning rows.
///
/// Column patterns and predicates are resolved against the original table.
/// Predicates are evaluated before excluded columns are removed, so a helper
/// column may select rows without becoming a model feature. Retained rows and
/// columns preserve their original order.
///
/// # Errors
///
/// Returns the same typed selection failures as training preparation for
/// duplicate names, unmatched patterns, invalid predicates, missing predicate
/// values, an empty column set, or an empty row set.
pub fn select_table(
	table: &RawTable,
	excluded_columns: impl IntoIterator<Item = ColumnPattern>,
	excluded_rows: impl IntoIterator<Item = RowPredicate>,
) -> PrepareResult<RawTable> {
	let request = PreparationRequest::new(
		core::iter::empty::<Vec<u8>>(),
		TrainFraction::new(1, 2).expect("one half is a valid internal selection fraction"),
	)
	.exclude_columns(excluded_columns)
	.exclude_rows(excluded_rows);
	let names = build_header_name_index(table)?;
	let excluded_columns = resolve_column_exclusions_from_headers(&request, table.headers(), &BTreeSet::new())?;
	if excluded_columns.len() == table.headers().len() {
		return Err(PrepareError::new(
			PrepareErrorKind::NoFeatureVectors,
			"column selection retained no vectors",
		));
	}
	let predicates = resolve_predicates_before_fit(&request, &names)?;
	let (retained_source_rows, _) = filter_rows(table, &predicates)?;
	if retained_source_rows.is_empty() {
		return Err(PrepareError::new(
			PrepareErrorKind::NoRetainedRows,
			"row exclusions retained no source rows",
		));
	}
	let retained_columns = (0..table.headers().len())
		.filter(|index| !excluded_columns.contains(index))
		.collect::<Vec<_>>();
	let headers = retained_columns
		.iter()
		.map(|index| table.headers()[*index].clone())
		.collect::<Vec<_>>();
	let rows = retained_source_rows
		.into_iter()
		.map(|source_row| {
			retained_columns
				.iter()
				.map(|column| table.rows()[source_row][*column].clone())
				.collect::<Vec<_>>()
		})
		.collect::<Vec<_>>();
	RawTable::from_parts(headers, rows).map_err(|error| {
		PrepareError::new(
			PrepareErrorKind::InconsistentInference,
			format!("selected table is not rectangular: {error}"),
		)
	})
}

type SelectedRowsAndColumns = (ResolvedTargets, BTreeSet<usize>, Vec<usize>, Vec<usize>);

fn select_rows_and_columns(
	table: &RawTable,
	inferred: &InferredVectorList,
	request: &PreparationRequest,
) -> PrepareResult<SelectedRowsAndColumns> {
	let names = build_name_index(inferred)?;
	let targets = resolve_targets(request, &names)?;
	let excluded_columns = resolve_column_exclusions(request, inferred, &targets.indices)?;
	if inferred
		.vectors()
		.iter()
		.all(|vector| targets.indices.contains(&vector.index()) || excluded_columns.contains(&vector.index()))
	{
		return Err(PrepareError::new(
			PrepareErrorKind::NoFeatureVectors,
			"column selection retained no feature vectors",
		));
	}
	let predicates = resolve_predicates(request, inferred, &names)?;
	let (retained_source_rows, excluded_source_rows) = filter_rows(table, &predicates)?;
	if retained_source_rows.is_empty() {
		return Err(PrepareError::new(
			PrepareErrorKind::NoRetainedRows,
			"row exclusions retained no source rows",
		));
	}
	Ok((
		targets,
		excluded_columns,
		retained_source_rows,
		excluded_source_rows,
	))
}

fn select_rows_and_columns_before_fit(
	table: &RawTable,
	request: &PreparationRequest,
) -> PrepareResult<SelectedRowsAndColumns> {
	let names = build_header_name_index(table)?;
	let targets = resolve_targets(request, &names)?;
	let excluded_columns = resolve_column_exclusions_from_headers(request, table.headers(), &targets.indices)?;
	if table
		.headers()
		.iter()
		.enumerate()
		.all(|(index, _)| targets.indices.contains(&index) || excluded_columns.contains(&index))
	{
		return Err(PrepareError::new(
			PrepareErrorKind::NoFeatureVectors,
			"column selection retained no feature vectors",
		));
	}
	let predicates = resolve_predicates_before_fit(request, &names)?;
	let (retained_source_rows, excluded_source_rows) = filter_rows(table, &predicates)?;
	if retained_source_rows.is_empty() {
		return Err(PrepareError::new(
			PrepareErrorKind::NoRetainedRows,
			"row exclusions retained no source rows",
		));
	}
	Ok((
		targets,
		excluded_columns,
		retained_source_rows,
		excluded_source_rows,
	))
}

fn fit_partition_table(table: &RawTable, fit_source_rows: &[usize]) -> PrepareResult<RawTable> {
	let rows = fit_source_rows
		.iter()
		.map(|source_row| {
			table.rows().get(*source_row).cloned().ok_or_else(|| {
				PrepareError::new(
					PrepareErrorKind::InconsistentInference,
					"fit partition references a source row outside the table",
				)
				.for_row(*source_row)
			})
		})
		.collect::<PrepareResult<Vec<_>>>()?;
	RawTable::from_parts(table.headers().to_vec(), rows).map_err(|error| {
		PrepareError::new(
			PrepareErrorKind::InconsistentInference,
			format!("cannot construct the fit-partition semantic view: {error}"),
		)
	})
}

fn fit_semantics_with_predicate_constraints(
	fit_table: &RawTable,
	request: &PreparationRequest,
	semantics: &[VectorSemanticRule],
) -> PrepareResult<Vec<VectorSemanticRule>> {
	let names = build_header_name_index(fit_table)?;
	let mut fitted = (0..fit_table.width())
		.map(|index| {
			semantics
				.get(index)
				.copied()
				.unwrap_or(VectorSemanticRule::Infer)
		})
		.collect::<Vec<_>>();
	for predicate in &request.excluded_rows {
		let Some((semantic_type, encoding)) = (match predicate.literal {
			PredicateLiteral::Signed(_) | PredicateLiteral::Unsigned(_) => {
				Some((SemanticType::Numeric, VectorEncoding::I32))
			}
			PredicateLiteral::F32Bits(_) => Some((SemanticType::Numeric, VectorEncoding::F32)),
			PredicateLiteral::Text(_) => Some((SemanticType::Text, VectorEncoding::Utf8)),
		}) else {
			continue;
		};
		let index = names.get(&predicate.column).copied().ok_or_else(|| {
			PrepareError::new(
				PrepareErrorKind::PredicateColumnNotFound,
				"row predicate column does not exist in fit table",
			)
			.for_column(&predicate.column)
		})?;
		let has_fit_value = fit_table
			.rows()
			.iter()
			.any(|row| row.get(index).is_some_and(|value| !value.is_empty()));
		if !has_fit_value && fitted[index] == VectorSemanticRule::Infer {
			fitted[index] = VectorSemanticRule::Exact(VectorSemantic {
				semantic_type,
				encoding,
			});
		}
	}
	Ok(fitted)
}

fn prepare_preselected_table(
	table: &RawTable,
	inferred: &InferredVectorList,
	targets: &ResolvedTargets,
	excluded_columns: &BTreeSet<usize>,
	retained_source_rows: Vec<usize>,
	excluded_source_rows: Vec<usize>,
	train_rows: usize,
) -> PrepareResult<PreparedDataset> {
	let fit_source_rows = &retained_source_rows[..train_rows];
	let schemas = inferred
		.vectors()
		.iter()
		.filter(|vector| !excluded_columns.contains(&vector.index()))
		.map(|vector| {
			fit_vector_schema(
				table,
				vector,
				if targets.indices.contains(&vector.index()) {
					VectorRole::Target
				} else {
					VectorRole::Feature
				},
				fit_source_rows,
			)
		})
		.collect::<PrepareResult<Vec<_>>>()?;
	let vectors = schemas
		.iter()
		.map(|schema| apply_vector_schema(table, schema, &retained_source_rows))
		.collect::<PrepareResult<Vec<_>>>()?;
	let train = partition(PartitionKind::Train, 0..train_rows, &retained_source_rows);
	let validation = partition(
		PartitionKind::Validation,
		train_rows..retained_source_rows.len(),
		&retained_source_rows,
	);
	Ok(PreparedDataset {
		source_row_count: table.rows().len(),
		retained_source_rows,
		excluded_source_rows,
		vectors,
		target_source_indices: targets.ordered.clone(),
		train,
		validation,
	})
}

fn semantic_error(error: SemanticError) -> PrepareError {
	PrepareError::new(PrepareErrorKind::SemanticInference, error.to_string())
}

fn validate_inference(table: &RawTable, inferred: &InferredVectorList) -> PrepareResult<()> {
	if inferred.vectors().len() != table.width() {
		return Err(PrepareError::new(
			PrepareErrorKind::InconsistentInference,
			format!(
				"inference contains {} vectors, table width is {}",
				inferred.vectors().len(),
				table.width()
			),
		));
	}
	for (index, vector) in inferred.vectors().iter().enumerate() {
		let expected_name = table.headers().get(index).ok_or_else(|| {
			PrepareError::new(
				PrepareErrorKind::InconsistentInference,
				format!("table has no header for inferred vector {index}"),
			)
		})?;
		if vector.index() != index || vector.name() != expected_name {
			return Err(PrepareError::new(
				PrepareErrorKind::InconsistentInference,
				format!("inferred vector {index} does not describe the corresponding source column"),
			)
			.for_column(expected_name));
		}
	}
	Ok(())
}

fn build_name_index(inferred: &InferredVectorList) -> PrepareResult<BTreeMap<Vec<u8>, usize>> {
	let mut names = BTreeMap::new();
	for vector in inferred.vectors() {
		if names
			.insert(vector.name().to_vec(), vector.index())
			.is_some()
		{
			return Err(PrepareError::new(
				PrepareErrorKind::DuplicateColumnName,
				"source column names must be unique for named selection",
			)
			.for_column(vector.name()));
		}
	}
	Ok(names)
}

fn build_header_name_index(table: &RawTable) -> PrepareResult<BTreeMap<Vec<u8>, usize>> {
	let mut names = BTreeMap::new();
	for (index, name) in table.headers().iter().enumerate() {
		if names.insert(name.clone(), index).is_some() {
			return Err(PrepareError::new(
				PrepareErrorKind::DuplicateColumnName,
				"source column names must be unique for named selection",
			)
			.for_column(name));
		}
	}
	Ok(names)
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ResolvedTargets {
	ordered: Vec<usize>,
	indices: BTreeSet<usize>,
}

fn resolve_targets(request: &PreparationRequest, names: &BTreeMap<Vec<u8>, usize>) -> PrepareResult<ResolvedTargets> {
	if request.targets.is_empty() {
		return Err(PrepareError::new(
			PrepareErrorKind::EmptyTargetSet,
			"at least one target column is required",
		));
	}
	let mut ordered = Vec::with_capacity(request.targets.len());
	let mut indices = BTreeSet::new();
	for name in &request.targets {
		let index = names.get(name).copied().ok_or_else(|| {
			PrepareError::new(
				PrepareErrorKind::TargetNotFound,
				"target column does not exist",
			)
			.for_column(name)
		})?;
		if !indices.insert(index) {
			return Err(PrepareError::new(
				PrepareErrorKind::DuplicateTarget,
				"target column was declared more than once",
			)
			.for_column(name));
		}
		ordered.push(index);
	}
	Ok(ResolvedTargets { ordered, indices })
}

fn resolve_column_exclusions(
	request: &PreparationRequest,
	inferred: &InferredVectorList,
	targets: &BTreeSet<usize>,
) -> PrepareResult<BTreeSet<usize>> {
	let mut excluded = BTreeSet::new();
	for pattern in &request.excluded_columns {
		let matches = inferred
			.vectors()
			.iter()
			.filter(|vector| pattern.matches(vector.name()))
			.map(InferredVector::index)
			.collect::<Vec<_>>();
		if matches.is_empty() {
			return Err(PrepareError::new(
				PrepareErrorKind::UnmatchedColumnPattern,
				format!(
					"column pattern {:?} matched no source vector",
					String::from_utf8_lossy(pattern.as_bytes())
				),
			));
		}
		for index in matches {
			if targets.contains(&index) {
				let name = inferred.vectors()[index].name();
				return Err(PrepareError::new(
					PrepareErrorKind::TargetExcluded,
					"column exclusion pattern also matched a target",
				)
				.for_column(name));
			}
			excluded.insert(index);
		}
	}
	Ok(excluded)
}

fn resolve_column_exclusions_from_headers(
	request: &PreparationRequest,
	headers: &[Vec<u8>],
	targets: &BTreeSet<usize>,
) -> PrepareResult<BTreeSet<usize>> {
	let mut excluded = BTreeSet::new();
	for pattern in &request.excluded_columns {
		let matches = headers
			.iter()
			.enumerate()
			.filter(|(_, name)| pattern.matches(name))
			.map(|(index, _)| index)
			.collect::<Vec<_>>();
		if matches.is_empty() {
			return Err(PrepareError::new(
				PrepareErrorKind::UnmatchedColumnPattern,
				format!(
					"column pattern {:?} matched no source vector",
					String::from_utf8_lossy(pattern.as_bytes())
				),
			));
		}
		for index in matches {
			if targets.contains(&index) {
				return Err(PrepareError::new(
					PrepareErrorKind::TargetExcluded,
					"column exclusion pattern also matched a target",
				)
				.for_column(&headers[index]));
			}
			excluded.insert(index);
		}
	}
	Ok(excluded)
}

struct ResolvedPredicate<'a> {
	predicate: &'a RowPredicate,
	column_index: usize,
	semantic_type: SemanticType,
	encoding: VectorEncoding,
	fitted_type: bool,
}

fn resolve_predicates<'a>(
	request: &'a PreparationRequest,
	inferred: &InferredVectorList,
	names: &BTreeMap<Vec<u8>, usize>,
) -> PrepareResult<Vec<ResolvedPredicate<'a>>> {
	request
		.excluded_rows
		.iter()
		.map(|predicate| {
			let column_index = names.get(&predicate.column).copied().ok_or_else(|| {
				PrepareError::new(
					PrepareErrorKind::PredicateColumnNotFound,
					"row predicate column does not exist",
				)
				.for_column(&predicate.column)
			})?;
			let vector = &inferred.vectors()[column_index];
			let semantic_type = vector.semantic_type();
			let encoding = vector.encoding();
			validate_predicate_type(predicate, semantic_type, encoding)?;
			Ok(ResolvedPredicate {
				predicate,
				column_index,
				semantic_type,
				encoding,
				fitted_type: true,
			})
		})
		.collect()
}

fn resolve_predicates_before_fit<'a>(
	request: &'a PreparationRequest,
	names: &BTreeMap<Vec<u8>, usize>,
) -> PrepareResult<Vec<ResolvedPredicate<'a>>> {
	request
		.excluded_rows
		.iter()
		.map(|predicate| {
			let column_index = names.get(&predicate.column).copied().ok_or_else(|| {
				PrepareError::new(
					PrepareErrorKind::PredicateColumnNotFound,
					"row predicate column does not exist",
				)
				.for_column(&predicate.column)
			})?;
			let (semantic_type, encoding) = match predicate.literal {
				PredicateLiteral::Signed(_) | PredicateLiteral::Unsigned(_) => {
					(SemanticType::Numeric, VectorEncoding::I32)
				}
				PredicateLiteral::F32Bits(bits) => {
					if !f32::from_bits(bits).is_finite() {
						return Err(PrepareError::new(
							PrepareErrorKind::InvalidPredicateLiteral,
							"f32 predicate literal must be finite",
						)
						.for_column(&predicate.column));
					}
					(SemanticType::Numeric, VectorEncoding::F32)
				}
				PredicateLiteral::Text(_) => (SemanticType::Text, VectorEncoding::Utf8),
			};
			Ok(ResolvedPredicate {
				predicate,
				column_index,
				semantic_type,
				encoding,
				fitted_type: false,
			})
		})
		.collect()
}

fn validate_predicates_after_fit(request: &PreparationRequest, inferred: &InferredVectorList) -> PrepareResult<()> {
	let names = build_name_index(inferred)?;
	for predicate in &request.excluded_rows {
		let index = names.get(&predicate.column).copied().ok_or_else(|| {
			PrepareError::new(
				PrepareErrorKind::PredicateColumnNotFound,
				"row predicate column does not exist in fitted semantics",
			)
			.for_column(&predicate.column)
		})?;
		let vector = &inferred.vectors()[index];
		validate_predicate_type(predicate, vector.semantic_type(), vector.encoding())?;
	}
	Ok(())
}

fn validate_predicate_type(
	predicate: &RowPredicate,
	semantic_type: SemanticType,
	encoding: VectorEncoding,
) -> PrepareResult<()> {
	let compatible = match predicate.literal {
		PredicateLiteral::Signed(_) | PredicateLiteral::Unsigned(_) => {
			semantic_type == SemanticType::Numeric && encoding == VectorEncoding::I32
		}
		PredicateLiteral::F32Bits(_) => semantic_type == SemanticType::Numeric && encoding == VectorEncoding::F32,
		PredicateLiteral::Text(_) => {
			matches!(
				semantic_type,
				SemanticType::Categorical | SemanticType::Ordinal | SemanticType::Text
			)
		}
	};
	if !compatible {
		return Err(PrepareError::new(
			PrepareErrorKind::PredicateTypeMismatch,
			format!(
				"predicate literal {:?} is incompatible with {semantic_type:?}/{encoding:?}",
				predicate.literal,
			),
		)
		.for_column(&predicate.column));
	}
	if let PredicateLiteral::F32Bits(bits) = predicate.literal
		&& !f32::from_bits(bits).is_finite()
	{
		return Err(PrepareError::new(
			PrepareErrorKind::InvalidPredicateLiteral,
			"f32 predicate literal must be finite",
		)
		.for_column(&predicate.column));
	}
	Ok(())
}

fn filter_rows(table: &RawTable, predicates: &[ResolvedPredicate<'_>]) -> PrepareResult<(Vec<usize>, Vec<usize>)> {
	let mut retained = Vec::with_capacity(table.rows().len());
	let mut excluded = Vec::new();
	for (source_row, row) in table.rows().iter().enumerate() {
		let mut exclude = false;
		for resolved in predicates {
			if exclude {
				break;
			}
			let value = row.get(resolved.column_index).ok_or_else(|| {
				PrepareError::new(
					PrepareErrorKind::InconsistentInference,
					"source row has no value for predicate column",
				)
				.for_column(&resolved.predicate.column)
				.for_row(source_row)
			})?;
			exclude = evaluate_predicate(value, source_row, resolved)?;
		}
		if exclude {
			excluded.push(source_row);
		} else {
			retained.push(source_row);
		}
	}
	Ok((retained, excluded))
}

fn evaluate_predicate(value: &[u8], source_row: usize, resolved: &ResolvedPredicate<'_>) -> PrepareResult<bool> {
	if value.is_empty() {
		return Err(PrepareError::new(
			PrepareErrorKind::MissingPredicateValue,
			"row predicate cannot compare a missing source value",
		)
		.for_column(&resolved.predicate.column)
		.for_row(source_row));
	}
	let ordering = match &resolved.predicate.literal {
		PredicateLiteral::Signed(literal) => {
			parse_predicate_text(value, source_row, resolved)?
				.parse::<i64>()
				.map(|parsed| parsed.cmp(literal))
				.map_err(|error| predicate_value_error(source_row, resolved, error))?
		}
		PredicateLiteral::Unsigned(literal) => {
			parse_predicate_text(value, source_row, resolved)?
				.parse::<u64>()
				.map(|parsed| parsed.cmp(literal))
				.map_err(|error| predicate_value_error(source_row, resolved, error))?
		}
		PredicateLiteral::F32Bits(bits) => {
			let parsed = parse_contract_f32(parse_predicate_text(value, source_row, resolved)?)
				.map_err(|error| predicate_value_error(source_row, resolved, error))?
				.value();
			parsed.partial_cmp(&f32::from_bits(*bits)).ok_or_else(|| {
				PrepareError::new(
					PrepareErrorKind::InvalidPredicateValue,
					"finite f32 comparison unexpectedly had no ordering",
				)
				.for_column(&resolved.predicate.column)
				.for_row(source_row)
			})?
		}
		PredicateLiteral::Text(literal) => parse_predicate_text(value, source_row, resolved)?.cmp(literal),
	};
	Ok(compare_ordering(ordering, resolved.predicate.operator))
}

fn parse_predicate_text<'a>(
	value: &'a [u8],
	source_row: usize,
	resolved: &ResolvedPredicate<'_>,
) -> PrepareResult<&'a str> {
	core::str::from_utf8(value).map_err(|error| predicate_value_error(source_row, resolved, error))
}

fn predicate_value_error(
	source_row: usize,
	resolved: &ResolvedPredicate<'_>,
	error: impl fmt::Display,
) -> PrepareError {
	PrepareError::new(
		if resolved.fitted_type {
			PrepareErrorKind::InvalidPredicateValue
		} else {
			PrepareErrorKind::PredicateTypeMismatch
		},
		format!(
			"cannot compare {:?}/{:?} source value using {:?}: {error}",
			resolved.semantic_type, resolved.encoding, resolved.predicate.literal
		),
	)
	.for_column(&resolved.predicate.column)
	.for_row(source_row)
}

const fn compare_ordering(ordering: Ordering, operator: ComparisonOperator) -> bool {
	match operator {
		ComparisonOperator::Equal => ordering.is_eq(),
		ComparisonOperator::NotEqual => !ordering.is_eq(),
		ComparisonOperator::Less => ordering.is_lt(),
		ComparisonOperator::LessOrEqual => !ordering.is_gt(),
		ComparisonOperator::Greater => ordering.is_gt(),
		ComparisonOperator::GreaterOrEqual => !ordering.is_lt(),
	}
}

fn fit_vector_schema(
	table: &RawTable,
	inferred: &InferredVector,
	role: VectorRole,
	fit_source_rows: &[usize],
) -> PrepareResult<VectorSchema> {
	let fit_values = vector_values(table, inferred.index(), inferred.name(), fit_source_rows)?;
	let metadata = match inferred.encoding() {
		VectorEncoding::I32 | VectorEncoding::F32 | VectorEncoding::Utf8 => VectorMetadata::None,
		VectorEncoding::RelativeSecondsI32 => {
			VectorMetadata::Temporal {
				origin: fit_temporal_origin(inferred, fit_source_rows, &fit_values)?,
			}
		}
		VectorEncoding::DictionaryI32 => {
			VectorMetadata::Categorical {
				dictionary: fit_dictionary(inferred, &fit_values)?,
			}
		}
		VectorEncoding::OrdinalI32 => {
			let present = fit_values
				.iter()
				.copied()
				.filter(|value| !value.is_empty())
				.collect::<Vec<_>>();
			let ordered_labels = if present.is_empty() {
				Vec::new()
			} else {
				fit_ordinal_vocabulary(&present)
					.ok_or_else(|| {
						PrepareError::new(
							PrepareErrorKind::EncodingFailure,
							"fit partition does not identify one recognized ordinal vocabulary",
						)
						.for_column(inferred.name())
					})?
					.iter()
					.map(|label| label.to_vec())
					.collect()
			};
			VectorMetadata::Ordinal { ordered_labels }
		}
		VectorEncoding::Bytes => {
			if inferred.semantic_type() == SemanticType::Image {
				VectorMetadata::Image {
					encoded_variants: inspect_image_variants(inferred, fit_source_rows, &fit_values)?,
				}
			} else {
				VectorMetadata::None
			}
		}
	};
	Ok(VectorSchema {
		source_index: inferred.index(),
		name: inferred.name().to_vec(),
		role,
		semantic_type: inferred.semantic_type(),
		encoding: inferred.encoding(),
		metadata,
	})
}

fn apply_vector_schema(
	table: &RawTable,
	schema: &VectorSchema,
	retained_source_rows: &[usize],
) -> PrepareResult<PreparedVector> {
	let values = vector_values(
		table,
		schema.source_index,
		&schema.name,
		retained_source_rows,
	)?;
	let (prepared_values, categorical_observations) = match (schema.encoding, &schema.metadata) {
		(VectorEncoding::I32, VectorMetadata::None) => {
			(
				PreparedValues::I32(encode_i32(schema, retained_source_rows, &values)?),
				None,
			)
		}
		(VectorEncoding::F32, VectorMetadata::None) => {
			(
				PreparedValues::F32Bits(encode_f32(schema, retained_source_rows, &values)?),
				None,
			)
		}
		(VectorEncoding::RelativeSecondsI32, VectorMetadata::Temporal { origin }) => {
			(
				PreparedValues::I32(encode_temporal(
					schema,
					retained_source_rows,
					&values,
					*origin,
				)?),
				None,
			)
		}
		(VectorEncoding::DictionaryI32, VectorMetadata::Categorical { dictionary }) => {
			let (codes, observations) = encode_dictionary(schema, retained_source_rows, &values, dictionary)?;
			validate_categorical_alignment(schema, dictionary.len(), &codes, &observations)?;
			(PreparedValues::I32(codes), Some(observations))
		}
		(VectorEncoding::OrdinalI32, VectorMetadata::Ordinal { ordered_labels }) => {
			(
				PreparedValues::I32(encode_ordinal(
					schema,
					retained_source_rows,
					&values,
					ordered_labels,
				)?),
				None,
			)
		}
		(VectorEncoding::Utf8, VectorMetadata::None) => {
			(
				PreparedValues::VariableWidth(encode_variable(schema, retained_source_rows, &values)?),
				None,
			)
		}
		(VectorEncoding::Bytes, VectorMetadata::Image { .. }) if schema.semantic_type == SemanticType::Image => {
			// Applying a schema still validates every encoded image header, but
			// validation-only variants are deliberately not added to fitted metadata.
			inspect_image_variants_for_schema(schema, retained_source_rows, &values)?;
			(
				PreparedValues::VariableWidth(encode_variable(schema, retained_source_rows, &values)?),
				None,
			)
		}
		(VectorEncoding::Bytes, VectorMetadata::None) => {
			(
				PreparedValues::VariableWidth(encode_variable(schema, retained_source_rows, &values)?),
				None,
			)
		}
		_ => {
			return Err(PrepareError::new(
				PrepareErrorKind::InconsistentPreparedVector,
				format!(
					"fitted {:?}/{:?} vector has incompatible metadata {:?}",
					schema.semantic_type, schema.encoding, schema.metadata
				),
			)
			.for_column(&schema.name));
		}
	};
	if prepared_values.len() != retained_source_rows.len()
		|| categorical_observations
			.as_ref()
			.is_some_and(|observations| observations.len() != retained_source_rows.len())
	{
		return Err(PrepareError::new(
			PrepareErrorKind::InconsistentPreparedVector,
			"schema application did not preserve retained-row length",
		)
		.for_column(&schema.name));
	}
	Ok(PreparedVector {
		source_index: schema.source_index,
		name: schema.name.clone(),
		role: schema.role,
		semantic_type: schema.semantic_type,
		encoding: schema.encoding,
		metadata: schema.metadata.clone(),
		values: prepared_values,
		categorical_observations,
	})
}

fn vector_values<'a>(
	table: &'a RawTable,
	source_index: usize,
	name: &[u8],
	source_rows: &[usize],
) -> PrepareResult<Vec<&'a [u8]>> {
	source_rows
		.iter()
		.map(|source_row| {
			table.rows()
				.get(*source_row)
				.and_then(|row| row.get(source_index))
				.map(Vec::as_slice)
				.ok_or_else(|| {
					PrepareError::new(
						PrepareErrorKind::InconsistentInference,
						"source row does not contain the fitted vector",
					)
					.for_column(name)
					.for_row(*source_row)
				})
		})
		.collect()
}

fn encode_i32(schema: &VectorSchema, source_rows: &[usize], values: &[&[u8]]) -> PrepareResult<Vec<Option<i32>>> {
	values.iter()
		.zip(source_rows)
		.map(|(value, source_row)| {
			if value.is_empty() {
				Ok(None)
			} else {
				let text = core::str::from_utf8(value)
					.map_err(|error| encoding_error(schema, *source_row, error))?;
				parse_contract_i32(text)
					.map(|value| Some(value.value()))
					.map_err(|error| encoding_error(schema, *source_row, error))
			}
		})
		.collect()
}

fn encode_f32(schema: &VectorSchema, source_rows: &[usize], values: &[&[u8]]) -> PrepareResult<Vec<Option<u32>>> {
	values.iter()
		.zip(source_rows)
		.map(|(value, source_row)| {
			if value.is_empty() {
				Ok(None)
			} else {
				let text = core::str::from_utf8(value)
					.map_err(|error| encoding_error(schema, *source_row, error))?;
				parse_contract_f32(text)
					.map(|value| Some(value.bits()))
					.map_err(|error| encoding_error(schema, *source_row, error))
			}
		})
		.collect()
}

fn fit_temporal_origin(
	inferred: &InferredVector,
	source_rows: &[usize],
	values: &[&[u8]],
) -> PrepareResult<TemporalOrigin> {
	let parsed = values
		.iter()
		.zip(source_rows)
		.map(|(value, source_row)| {
			if value.is_empty() {
				Ok(None)
			} else {
				parse_temporal_instant(value)
					.map(Some)
					.ok_or_else(|| inferred_encoding_error(inferred, *source_row, "invalid temporal value"))
			}
		})
		.collect::<PrepareResult<Vec<_>>>()?;
	let origin = parsed
		.iter()
		.flatten()
		.copied()
		.min()
		.unwrap_or(TemporalInstant {
			unix_seconds: 0,
			nanoseconds: 0,
		});
	Ok(TemporalOrigin {
		unix_seconds: origin.unix_seconds,
		nanoseconds: origin.nanoseconds,
	})
}

fn encode_temporal(
	schema: &VectorSchema,
	source_rows: &[usize],
	values: &[&[u8]],
	origin: TemporalOrigin,
) -> PrepareResult<Vec<Option<i32>>> {
	let origin = TemporalInstant {
		unix_seconds: origin.unix_seconds,
		nanoseconds: origin.nanoseconds,
	};
	let parsed = values
		.iter()
		.zip(source_rows)
		.map(|(value, source_row)| {
			if value.is_empty() {
				Ok(None)
			} else {
				parse_temporal_instant(value)
					.map(Some)
					.ok_or_else(|| encoding_error(schema, *source_row, "invalid temporal value"))
			}
		})
		.collect::<PrepareResult<Vec<_>>>()?;
	let values = parsed
		.into_iter()
		.zip(source_rows)
		.map(|(instant, source_row)| {
			instant
				.map(|instant| temporal_delta(schema, origin, instant, *source_row))
				.transpose()
		})
		.collect::<PrepareResult<Vec<_>>>()?;
	Ok(values)
}

fn temporal_delta(
	schema: &VectorSchema,
	origin: TemporalInstant,
	value: TemporalInstant,
	source_row: usize,
) -> PrepareResult<i32> {
	let second_delta = i128::from(value.unix_seconds) - i128::from(origin.unix_seconds);
	let nanosecond_delta = i128::from(value.nanoseconds) - i128::from(origin.nanoseconds);
	let total_nanoseconds = second_delta
		.checked_mul(1_000_000_000)
		.and_then(|seconds| seconds.checked_add(nanosecond_delta))
		.ok_or_else(|| {
			PrepareError::new(
				PrepareErrorKind::ArithmeticOverflow,
				"relative temporal nanoseconds overflowed i128",
			)
			.for_column(&schema.name)
			.for_row(source_row)
		})?;
	if total_nanoseconds % 1_000_000_000 != 0 {
		return Err(PrepareError::new(
			PrepareErrorKind::EncodingFailure,
			"temporal values do not share a lossless whole-second offset from the retained origin",
		)
		.for_column(&schema.name)
		.for_row(source_row));
	}
	let relative_seconds = total_nanoseconds / 1_000_000_000;
	i32::try_from(relative_seconds).map_err(|error| {
		PrepareError::new(
			PrepareErrorKind::TemporalRangeExceeded,
			format!("relative temporal seconds exceed i32: {error}"),
		)
		.for_column(&schema.name)
		.for_row(source_row)
	})
}

fn fit_dictionary(inferred: &InferredVector, values: &[&[u8]]) -> PrepareResult<Vec<Vec<u8>>> {
	let dictionary = values
		.iter()
		.copied()
		.filter(|value| !value.is_empty())
		.map(<[u8]>::to_vec)
		.collect::<BTreeSet<_>>()
		.into_iter()
		.collect::<Vec<_>>();
	i32::try_from(dictionary.len()).map_err(|error| {
		PrepareError::new(
			PrepareErrorKind::EncodingFailure,
			format!("categorical reserved code exceeds i32: {error}"),
		)
		.for_column(inferred.name())
	})?;
	Ok(dictionary)
}

type AppliedDictionary = (Vec<Option<i32>>, Vec<CategoricalObservation>);

fn validate_categorical_alignment(
	schema: &VectorSchema,
	dictionary_len: usize,
	codes: &[Option<i32>],
	observations: &[CategoricalObservation],
) -> PrepareResult<()> {
	let reserved = i32::try_from(dictionary_len).map_err(|error| {
		PrepareError::new(
			PrepareErrorKind::InconsistentPreparedVector,
			format!("categorical reserved code cannot be represented: {error}"),
		)
		.for_column(&schema.name)
	})?;
	if codes.len() != observations.len() {
		return Err(PrepareError::new(
			PrepareErrorKind::InconsistentPreparedVector,
			"categorical calculation codes and observation routes have different lengths",
		)
		.for_column(&schema.name));
	}
	for (code, observation) in codes.iter().zip(observations) {
		let aligned = match observation {
			CategoricalObservation::Known { code: known } => {
				*code == Some(*known) && *known >= 0 && *known < reserved
			}
			CategoricalObservation::Missing => code.is_none(),
			CategoricalObservation::Unseen { label } => !label.is_empty() && *code == Some(reserved),
		};
		if !aligned {
			return Err(PrepareError::new(
				PrepareErrorKind::InconsistentPreparedVector,
				"categorical calculation code does not match its typed observation route",
			)
			.for_column(&schema.name));
		}
	}
	Ok(())
}

fn encode_dictionary(
	schema: &VectorSchema,
	source_rows: &[usize],
	values: &[&[u8]],
	dictionary: &[Vec<u8>],
) -> PrepareResult<AppliedDictionary> {
	let known_codes = dictionary
		.iter()
		.enumerate()
		.map(|(index, value)| {
			i32::try_from(index)
				.map(|code| (value.as_slice(), code))
				.map_err(|error| {
					PrepareError::new(
						PrepareErrorKind::EncodingFailure,
						format!("categorical dictionary exceeds i32 codes: {error}"),
					)
					.for_column(&schema.name)
				})
		})
		.collect::<PrepareResult<BTreeMap<_, _>>>()?;
	let reserved_code = i32::try_from(dictionary.len()).map_err(|error| {
		PrepareError::new(
			PrepareErrorKind::EncodingFailure,
			format!("categorical reserved code exceeds i32: {error}"),
		)
		.for_column(&schema.name)
	})?;
	let encoded = values
		.iter()
		.zip(source_rows)
		.map(|(value, _source_row)| {
			if value.is_empty() {
				Ok((None, CategoricalObservation::Missing))
			} else if let Some(code) = known_codes.get(value).copied() {
				Ok((Some(code), CategoricalObservation::Known { code }))
			} else {
				Ok((Some(reserved_code), CategoricalObservation::Unseen {
					label: value.to_vec(),
				}))
			}
		})
		.collect::<PrepareResult<Vec<_>>>()?;
	let (codes, observations) = encoded.into_iter().unzip();
	Ok((codes, observations))
}

fn encode_ordinal(
	schema: &VectorSchema,
	source_rows: &[usize],
	values: &[&[u8]],
	vocabulary: &[Vec<u8>],
) -> PrepareResult<Vec<Option<i32>>> {
	values.iter()
		.zip(source_rows)
		.map(|(value, source_row)| {
			if value.is_empty() {
				return Ok(None);
			}
			let rank = vocabulary
				.iter()
				.position(|label| value.eq_ignore_ascii_case(label))
				.ok_or_else(|| {
					encoding_error(
						schema,
						*source_row,
						"value is absent from ordinal vocabulary",
					)
				})?;
			i32::try_from(rank)
				.map(Some)
				.map_err(|error| encoding_error(schema, *source_row, error))
		})
		.collect()
}

fn encode_variable(
	schema: &VectorSchema,
	source_rows: &[usize],
	values: &[&[u8]],
) -> PrepareResult<VariableWidthVector> {
	let mut offsets = Vec::with_capacity(values.len().saturating_add(1));
	let mut payload = Vec::new();
	let mut valid = Vec::with_capacity(values.len());
	offsets.push(0);
	for (value, source_row) in values.iter().zip(source_rows) {
		if schema.encoding == VectorEncoding::Utf8 && !value.is_empty() {
			core::str::from_utf8(value).map_err(|error| encoding_error(schema, *source_row, error))?;
		}
		valid.push(!value.is_empty());
		payload.extend_from_slice(value);
		offsets.push(u64::try_from(payload.len()).map_err(|error| {
			PrepareError::new(
				PrepareErrorKind::ArithmeticOverflow,
				format!("variable-width payload offset exceeds u64: {error}"),
			)
			.for_column(&schema.name)
			.for_row(*source_row)
		})?);
	}
	Ok(VariableWidthVector {
		offsets,
		payload,
		valid,
	})
}

fn inspect_image_variants(
	inferred: &InferredVector,
	source_rows: &[usize],
	values: &[&[u8]],
) -> PrepareResult<Vec<EncodedImageMetadata>> {
	let variants = values
		.iter()
		.zip(source_rows)
		.filter(|(value, _source_row)| !value.is_empty())
		.map(|(value, source_row)| {
			inspect_encoded_image(value).map_err(|error| {
				inferred_encoding_error(
					inferred,
					*source_row,
					format!("invalid encoded image header: {error}"),
				)
			})
		})
		.collect::<PrepareResult<BTreeSet<_>>>()?
		.into_iter()
		.collect::<Vec<_>>();
	Ok(variants)
}

fn inspect_image_variants_for_schema(
	schema: &VectorSchema,
	source_rows: &[usize],
	values: &[&[u8]],
) -> PrepareResult<()> {
	for (value, source_row) in values.iter().zip(source_rows) {
		if !value.is_empty() {
			inspect_encoded_image(value).map_err(|error| {
				encoding_error(
					schema,
					*source_row,
					format!("invalid encoded image header: {error}"),
				)
			})?;
		}
	}
	Ok(())
}

fn inferred_encoding_error(inferred: &InferredVector, source_row: usize, error: impl fmt::Display) -> PrepareError {
	encoding_error_parts(inferred.name(), inferred.encoding(), source_row, error)
}

fn encoding_error(schema: &VectorSchema, source_row: usize, error: impl fmt::Display) -> PrepareError {
	encoding_error_parts(&schema.name, schema.encoding, source_row, error)
}

fn encoding_error_parts(
	name: &[u8],
	encoding: VectorEncoding,
	source_row: usize,
	error: impl fmt::Display,
) -> PrepareError {
	PrepareError::new(
		PrepareErrorKind::EncodingFailure,
		format!("lossless {encoding:?} encoding failed: {error}"),
	)
	.for_column(name)
	.for_row(source_row)
}

fn partition(
	kind: PartitionKind,
	positions: core::ops::Range<usize>,
	retained_source_rows: &[usize],
) -> PreparedPartition {
	let retained_positions = positions.collect::<Vec<_>>();
	let source_rows = retained_positions
		.iter()
		.map(|position| retained_source_rows[*position])
		.collect();
	PreparedPartition {
		kind,
		retained_positions,
		source_rows,
	}
}

fn matrix_capacity(rows: usize, columns: usize) -> PrepareResult<usize> {
	rows.checked_mul(columns).ok_or_else(|| {
		PrepareError::new(
			PrepareErrorKind::ArithmeticOverflow,
			"dense matrix element count overflowed usize",
		)
	})
}

fn variable_dense_error(vector: &PreparedVector) -> PrepareError {
	PrepareError::new(
		PrepareErrorKind::VariableWidthDenseMatrix,
		format!(
			"{:?} vectors remain variable-width and cannot enter a fixed dense matrix",
			vector.semantic_type
		),
	)
	.for_column(&vector.name)
}

fn mixed_dense_error(vector: &PreparedVector, source_row: Option<usize>, value: i32) -> PrepareError {
	let error = PrepareError::new(
		PrepareErrorKind::MixedDenseEncoding,
		format!("int32 value {value} cannot be represented exactly in a mixed f32 dense matrix"),
	)
	.for_column(&vector.name);
	source_row.map_or(error.clone(), |row| error.for_row(row))
}

fn missing_dense_error(vector: &PreparedVector, source_row: Option<usize>) -> PrepareError {
	let error = PrepareError::new(
		PrepareErrorKind::MissingDenseValue,
		"fixed dense projection does not impute missing values",
	)
	.for_column(&vector.name);
	source_row.map_or(error.clone(), |row| error.for_row(row))
}

fn inconsistent_vector_error(vector: &PreparedVector, retained_position: usize) -> PrepareError {
	PrepareError::new(
		PrepareErrorKind::InconsistentPreparedVector,
		format!("prepared vector has no retained position {retained_position}"),
	)
	.for_column(&vector.name)
}

fn glob_matches(pattern: &[u8], value: &[u8]) -> bool {
	let mut pattern_index = 0usize;
	let mut value_index = 0usize;
	let mut star = None;
	let mut star_value = 0usize;
	while value_index < value.len() {
		if pattern
			.get(pattern_index)
			.is_some_and(|candidate| *candidate == b'?' || candidate.eq_ignore_ascii_case(&value[value_index]))
		{
			pattern_index += 1;
			value_index += 1;
		} else if pattern.get(pattern_index) == Some(&b'*') {
			star = Some(pattern_index);
			pattern_index += 1;
			star_value = value_index;
		} else if let Some(star_index) = star {
			pattern_index = star_index + 1;
			star_value += 1;
			value_index = star_value;
		} else {
			return false;
		}
	}
	while pattern.get(pattern_index) == Some(&b'*') {
		pattern_index += 1;
	}
	pattern_index == pattern.len()
}
