use core::cmp::Ordering;
use core::fmt;
use core::num::NonZeroU64;
use std::collections::{BTreeMap, BTreeSet};

use crate::image_header::{EncodedImageMetadata, inspect_encoded_image};
use crate::semantic::{TemporalInstant, ordinal_vocabulary, parse_temporal_instant};
use crate::{
	AmbiguousVectorModel, InferredVector, InferredVectorList, RawTable, SemanticError, SemanticType, VectorEncoding,
	infer_table_vectors, parse_contract_f32, parse_contract_i32,
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
	pub const fn numerator(self) -> u64 {
		self.numerator
	}

	#[must_use]
	pub const fn denominator(self) -> NonZeroU64 {
		self.denominator
	}

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

	fn try_from(value: f32) -> Result<Self, Self::Error> {
		Self::from_f32(value)
	}
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
	pub fn as_bytes(&self) -> &[u8] {
		&self.pattern
	}

	fn matches(&self, name: &[u8]) -> bool {
		glob_matches(&self.pattern, name)
	}
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
	pub const fn f32(value: f32) -> Self {
		Self::F32Bits(value.to_bits())
	}
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
	pub fn column(&self) -> &[u8] {
		&self.column
	}

	#[must_use]
	pub const fn operator(&self) -> ComparisonOperator {
		self.operator
	}

	#[must_use]
	pub const fn literal(&self) -> &PredicateLiteral {
		&self.literal
	}
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
	pub fn targets(&self) -> &[Vec<u8>] {
		&self.targets
	}

	#[must_use]
	pub fn excluded_columns(&self) -> &[ColumnPattern] {
		&self.excluded_columns
	}

	#[must_use]
	pub fn excluded_rows(&self) -> &[RowPredicate] {
		&self.excluded_rows
	}

	#[must_use]
	pub const fn train_fraction(&self) -> TrainFraction {
		self.train_fraction
	}
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
		/// Known code `n` denotes `dictionary[n]`. Code `dictionary.len()`
		/// is reserved for a nonempty value not observed in the fit partition.
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

/// Packed values for exactly one retained source vector.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PreparedValues {
	I32(Vec<Option<i32>>),
	/// Exact IEEE-754 bit patterns avoid accidental host-side arithmetic.
	F32Bits(Vec<Option<u32>>),
	VariableWidth(VariableWidthVector),
}

impl PreparedValues {
	#[must_use]
	pub fn len(&self) -> usize {
		match self {
			Self::I32(values) => values.len(),
			Self::F32Bits(values) => values.len(),
			Self::VariableWidth(values) => values.len(),
		}
	}

	#[must_use]
	pub fn is_empty(&self) -> bool {
		self.len() == 0
	}
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
	pub fn len(&self) -> usize {
		self.valid.len()
	}

	#[must_use]
	pub fn is_empty(&self) -> bool {
		self.valid.is_empty()
	}

	#[must_use]
	pub fn offsets(&self) -> &[u64] {
		&self.offsets
	}

	#[must_use]
	pub fn payload(&self) -> &[u8] {
		&self.payload
	}

	#[must_use]
	pub fn validity(&self) -> &[bool] {
		&self.valid
	}

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
	pub const fn source_index(&self) -> usize {
		self.source_index
	}

	#[must_use]
	pub fn name(&self) -> &[u8] {
		&self.name
	}

	#[must_use]
	pub const fn role(&self) -> VectorRole {
		self.role
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
	pub const fn metadata(&self) -> &VectorMetadata {
		&self.metadata
	}
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
	pub const fn source_index(&self) -> usize {
		self.source_index
	}

	#[must_use]
	pub fn name(&self) -> &[u8] {
		&self.name
	}

	#[must_use]
	pub const fn role(&self) -> VectorRole {
		self.role
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
	pub const fn metadata(&self) -> &VectorMetadata {
		&self.metadata
	}

	#[must_use]
	pub const fn values(&self) -> &PreparedValues {
		&self.values
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
	pub const fn kind(&self) -> PartitionKind {
		self.kind
	}

	#[must_use]
	pub fn retained_positions(&self) -> &[usize] {
		&self.retained_positions
	}

	#[must_use]
	pub fn source_rows(&self) -> &[usize] {
		&self.source_rows
	}

	#[must_use]
	pub fn len(&self) -> usize {
		self.retained_positions.len()
	}

	#[must_use]
	pub fn is_empty(&self) -> bool {
		self.retained_positions.is_empty()
	}
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
	train: PreparedPartition,
	validation: PreparedPartition,
}

impl PreparedDataset {
	#[must_use]
	pub const fn source_row_count(&self) -> usize {
		self.source_row_count
	}

	#[must_use]
	pub fn retained_source_rows(&self) -> &[usize] {
		&self.retained_source_rows
	}

	#[must_use]
	pub fn excluded_source_rows(&self) -> &[usize] {
		&self.excluded_source_rows
	}

	#[must_use]
	pub fn vectors(&self) -> &[PreparedVector] {
		&self.vectors
	}

	#[must_use]
	pub const fn train(&self) -> &PreparedPartition {
		&self.train
	}

	#[must_use]
	pub const fn validation(&self) -> &PreparedPartition {
		&self.validation
	}

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
					PreparedValues::F32Bits(values) => values
						.get(*position)
						.ok_or_else(|| inconsistent_vector_error(vector, *position))?
						.ok_or_else(|| missing_dense_error(vector, self.source_row_at(*position)))?,
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

/// Infer semantics and prepare a table without inventing derived features.
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
	let inferred = infer_table_vectors(table, model).map_err(semantic_error)?;
	prepare_inferred_table(table, &inferred, request)
}

/// Prepare a table using a caller-retained inference result.
///
/// Exactly one output vector is emitted for each source vector retained after
/// column exclusion. Rows are excluded before the exact rational split.
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
	let names = build_name_index(inferred)?;
	let target_indices = resolve_targets(request, &names)?;
	let excluded_columns = resolve_column_exclusions(request, inferred, &target_indices)?;
	if inferred
		.vectors()
		.iter()
		.all(|vector| target_indices.contains(&vector.index()) || excluded_columns.contains(&vector.index()))
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
	let train_rows = request
		.train_fraction
		.train_rows(retained_source_rows.len())?;
	let vectors = inferred
		.vectors()
		.iter()
		.filter(|vector| !excluded_columns.contains(&vector.index()))
		.map(|vector| {
			encode_vector(
				table,
				vector,
				if target_indices.contains(&vector.index()) {
					VectorRole::Target
				} else {
					VectorRole::Feature
				},
				&retained_source_rows,
				train_rows,
			)
		})
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

fn resolve_targets(request: &PreparationRequest, names: &BTreeMap<Vec<u8>, usize>) -> PrepareResult<BTreeSet<usize>> {
	if request.targets.is_empty() {
		return Err(PrepareError::new(
			PrepareErrorKind::EmptyTargetSet,
			"at least one target column is required",
		));
	}
	let mut targets = BTreeSet::new();
	for name in &request.targets {
		let index = names.get(name).copied().ok_or_else(|| {
			PrepareError::new(
				PrepareErrorKind::TargetNotFound,
				"target column does not exist",
			)
			.for_column(name)
		})?;
		if !targets.insert(index) {
			return Err(PrepareError::new(
				PrepareErrorKind::DuplicateTarget,
				"target column was declared more than once",
			)
			.for_column(name));
		}
	}
	Ok(targets)
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

struct ResolvedPredicate<'a> {
	predicate: &'a RowPredicate,
	column_index: usize,
	semantic_type: SemanticType,
	encoding: VectorEncoding,
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
			})
		})
		.collect()
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
		PredicateLiteral::Text(_) => matches!(
			semantic_type,
			SemanticType::Categorical | SemanticType::Ordinal | SemanticType::Text
		),
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
			let value = row.get(resolved.column_index).ok_or_else(|| {
				PrepareError::new(
					PrepareErrorKind::InconsistentInference,
					"source row has no value for predicate column",
				)
				.for_column(&resolved.predicate.column)
				.for_row(source_row)
			})?;
			exclude |= evaluate_predicate(value, source_row, resolved)?;
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
		PredicateLiteral::Signed(literal) => parse_predicate_text(value, source_row, resolved)?
			.parse::<i64>()
			.map(|parsed| parsed.cmp(literal))
			.map_err(|error| predicate_value_error(source_row, resolved, error))?,
		PredicateLiteral::Unsigned(literal) => parse_predicate_text(value, source_row, resolved)?
			.parse::<u64>()
			.map(|parsed| parsed.cmp(literal))
			.map_err(|error| predicate_value_error(source_row, resolved, error))?,
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
		PrepareErrorKind::InvalidPredicateValue,
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

fn encode_vector(
	table: &RawTable,
	inferred: &InferredVector,
	role: VectorRole,
	retained_source_rows: &[usize],
	train_rows: usize,
) -> PrepareResult<PreparedVector> {
	let source_values = table
		.rows()
		.iter()
		.map(|row| row[inferred.index()].as_slice())
		.collect::<Vec<_>>();
	let retained_values = retained_source_rows
		.iter()
		.map(|source_row| source_values[*source_row])
		.collect::<Vec<_>>();
	let (metadata, values) = match inferred.encoding() {
		VectorEncoding::I32 => (
			VectorMetadata::None,
			PreparedValues::I32(encode_i32(
				inferred,
				retained_source_rows,
				&retained_values,
			)?),
		),
		VectorEncoding::F32 => (
			VectorMetadata::None,
			PreparedValues::F32Bits(encode_f32(
				inferred,
				retained_source_rows,
				&retained_values,
			)?),
		),
		VectorEncoding::RelativeSecondsI32 => {
			let (origin, values) = encode_temporal(inferred, retained_source_rows, &retained_values)?;
			(
				VectorMetadata::Temporal { origin },
				PreparedValues::I32(values),
			)
		}
		VectorEncoding::DictionaryI32 => {
			let (dictionary, values) =
				encode_dictionary(inferred, retained_source_rows, &retained_values, train_rows)?;
			(
				VectorMetadata::Categorical { dictionary },
				PreparedValues::I32(values),
			)
		}
		VectorEncoding::OrdinalI32 => {
			let vocabulary = ordinal_vocabulary(
				&source_values
					.iter()
					.copied()
					.filter(|value| !value.is_empty())
					.collect::<Vec<_>>(),
			)
			.ok_or_else(|| {
				PrepareError::new(
					PrepareErrorKind::EncodingFailure,
					"inferred ordinal vector has no recognized order vocabulary",
				)
				.for_column(inferred.name())
			})?;
			let values = encode_ordinal(inferred, retained_source_rows, &retained_values, vocabulary)?;
			(
				VectorMetadata::Ordinal {
					ordered_labels: vocabulary.iter().map(|label| label.to_vec()).collect(),
				},
				PreparedValues::I32(values),
			)
		}
		VectorEncoding::Utf8 => (
			VectorMetadata::None,
			PreparedValues::VariableWidth(encode_variable(
				inferred,
				retained_source_rows,
				&retained_values,
			)?),
		),
		VectorEncoding::Bytes => {
			let metadata = if inferred.semantic_type() == SemanticType::Image {
				VectorMetadata::Image {
					encoded_variants: inspect_image_variants(
						inferred,
						retained_source_rows,
						&retained_values,
					)?,
				}
			} else {
				VectorMetadata::None
			};
			(
				metadata,
				PreparedValues::VariableWidth(encode_variable(
					inferred,
					retained_source_rows,
					&retained_values,
				)?),
			)
		}
	};
	Ok(PreparedVector {
		source_index: inferred.index(),
		name: inferred.name().to_vec(),
		role,
		semantic_type: inferred.semantic_type(),
		encoding: inferred.encoding(),
		metadata,
		values,
	})
}

fn encode_i32(inferred: &InferredVector, source_rows: &[usize], values: &[&[u8]]) -> PrepareResult<Vec<Option<i32>>> {
	values.iter()
		.zip(source_rows)
		.map(|(value, source_row)| {
			if value.is_empty() {
				Ok(None)
			} else {
				let text = core::str::from_utf8(value)
					.map_err(|error| encoding_error(inferred, *source_row, error))?;
				parse_contract_i32(text)
					.map(|value| Some(value.value()))
					.map_err(|error| encoding_error(inferred, *source_row, error))
			}
		})
		.collect()
}

fn encode_f32(inferred: &InferredVector, source_rows: &[usize], values: &[&[u8]]) -> PrepareResult<Vec<Option<u32>>> {
	values.iter()
		.zip(source_rows)
		.map(|(value, source_row)| {
			if value.is_empty() {
				Ok(None)
			} else {
				let text = core::str::from_utf8(value)
					.map_err(|error| encoding_error(inferred, *source_row, error))?;
				parse_contract_f32(text)
					.map(|value| Some(value.bits()))
					.map_err(|error| encoding_error(inferred, *source_row, error))
			}
		})
		.collect()
}

fn encode_temporal(
	inferred: &InferredVector,
	source_rows: &[usize],
	values: &[&[u8]],
) -> PrepareResult<(TemporalOrigin, Vec<Option<i32>>)> {
	let parsed = values
		.iter()
		.zip(source_rows)
		.map(|(value, source_row)| {
			if value.is_empty() {
				Ok(None)
			} else {
				parse_temporal_instant(value)
					.map(Some)
					.ok_or_else(|| encoding_error(inferred, *source_row, "invalid temporal value"))
			}
		})
		.collect::<PrepareResult<Vec<_>>>()?;
	let origin = parsed.iter().flatten().copied().min().ok_or_else(|| {
		PrepareError::new(
			PrepareErrorKind::EncodingFailure,
			"temporal vector has no retained nonmissing value",
		)
		.for_column(inferred.name())
	})?;
	let values = parsed
		.into_iter()
		.zip(source_rows)
		.map(|(instant, source_row)| {
			instant
				.map(|instant| temporal_delta(inferred, origin, instant, *source_row))
				.transpose()
		})
		.collect::<PrepareResult<Vec<_>>>()?;
	Ok((
		TemporalOrigin {
			unix_seconds: origin.unix_seconds,
			nanoseconds: origin.nanoseconds,
		},
		values,
	))
}

fn temporal_delta(
	inferred: &InferredVector,
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
			.for_column(inferred.name())
			.for_row(source_row)
		})?;
	if total_nanoseconds % 1_000_000_000 != 0 {
		return Err(PrepareError::new(
			PrepareErrorKind::EncodingFailure,
			"temporal values do not share a lossless whole-second offset from the retained origin",
		)
		.for_column(inferred.name())
		.for_row(source_row));
	}
	let relative_seconds = total_nanoseconds / 1_000_000_000;
	i32::try_from(relative_seconds).map_err(|error| {
		PrepareError::new(
			PrepareErrorKind::TemporalRangeExceeded,
			format!("relative temporal seconds exceed i32: {error}"),
		)
		.for_column(inferred.name())
		.for_row(source_row)
	})
}

type DictionaryEncoding = (Vec<Vec<u8>>, Vec<Option<i32>>);

fn encode_dictionary(
	inferred: &InferredVector,
	source_rows: &[usize],
	values: &[&[u8]],
	train_rows: usize,
) -> PrepareResult<DictionaryEncoding> {
	let dictionary = values
		.iter()
		.take(train_rows)
		.copied()
		.filter(|value| !value.is_empty())
		.map(<[u8]>::to_vec)
		.collect::<BTreeSet<_>>()
		.into_iter()
		.collect::<Vec<_>>();
	let codes = dictionary
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
					.for_column(inferred.name())
				})
		})
		.collect::<PrepareResult<BTreeMap<_, _>>>()?;
	let reserved_code = i32::try_from(dictionary.len()).map_err(|error| {
		PrepareError::new(
			PrepareErrorKind::EncodingFailure,
			format!("categorical reserved code exceeds i32: {error}"),
		)
		.for_column(inferred.name())
	})?;
	let encoded = values
		.iter()
		.zip(source_rows)
		.map(|(value, _source_row)| {
			if value.is_empty() {
				Ok(None)
			} else {
				Ok(Some(codes.get(value).copied().unwrap_or(reserved_code)))
			}
		})
		.collect::<PrepareResult<Vec<_>>>()?;
	Ok((dictionary, encoded))
}

fn encode_ordinal(
	inferred: &InferredVector,
	source_rows: &[usize],
	values: &[&[u8]],
	vocabulary: &[&[u8]],
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
						inferred,
						*source_row,
						"value is absent from ordinal vocabulary",
					)
				})?;
			i32::try_from(rank)
				.map(Some)
				.map_err(|error| encoding_error(inferred, *source_row, error))
		})
		.collect()
}

fn encode_variable(
	inferred: &InferredVector,
	source_rows: &[usize],
	values: &[&[u8]],
) -> PrepareResult<VariableWidthVector> {
	let mut offsets = Vec::with_capacity(values.len().saturating_add(1));
	let mut payload = Vec::new();
	let mut valid = Vec::with_capacity(values.len());
	offsets.push(0);
	for (value, source_row) in values.iter().zip(source_rows) {
		if inferred.encoding() == VectorEncoding::Utf8 && !value.is_empty() {
			core::str::from_utf8(value).map_err(|error| encoding_error(inferred, *source_row, error))?;
		}
		valid.push(!value.is_empty());
		payload.extend_from_slice(value);
		offsets.push(u64::try_from(payload.len()).map_err(|error| {
			PrepareError::new(
				PrepareErrorKind::ArithmeticOverflow,
				format!("variable-width payload offset exceeds u64: {error}"),
			)
			.for_column(inferred.name())
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
				encoding_error(
					inferred,
					*source_row,
					format!("invalid encoded image header: {error}"),
				)
			})
		})
		.collect::<PrepareResult<BTreeSet<_>>>()?
		.into_iter()
		.collect::<Vec<_>>();
	if variants.is_empty() {
		return Err(PrepareError::new(
			PrepareErrorKind::EncodingFailure,
			"image vector has no retained nonmissing encoded image",
		)
		.for_column(inferred.name()));
	}
	Ok(variants)
}

fn encoding_error(inferred: &InferredVector, source_row: usize, error: impl fmt::Display) -> PrepareError {
	PrepareError::new(
		PrepareErrorKind::EncodingFailure,
		format!(
			"lossless {:?} encoding failed: {error}",
			inferred.encoding()
		),
	)
	.for_column(inferred.name())
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

#[cfg(test)]
mod tests {
	use crate::{CategoricalEncodingModel, Delimiter, HeaderMode, IngestLimits, TableRequest, parse_table};

	use super::*;

	fn table(bytes: &[u8], records: u64) -> RawTable {
		parse_table(
			bytes,
			TableRequest::new(
				Delimiter::Comma,
				HeaderMode::Present,
				IngestLimits::new(1_048_576, records, 32, 1_024).unwrap(),
			),
		)
		.unwrap()
	}

	fn fraction() -> TrainFraction {
		TrainFraction::new(3, 4).unwrap()
	}

	fn jpeg(width: u16, height: u16) -> Vec<u8> {
		let mut bytes = b"\xff\xd8\xff\xe0\x00\x07JFIF\0\xff\xc0\x00\x11\x08".to_vec();
		bytes.extend_from_slice(&height.to_be_bytes());
		bytes.extend_from_slice(&width.to_be_bytes());
		bytes.extend_from_slice(&[3, 1, 0x11, 0, 2, 0x11, 0, 3, 0x11, 0]);
		bytes
	}

	fn gif(width: u16, height: u16) -> Vec<u8> {
		let mut bytes = b"GIF89a".to_vec();
		bytes.extend_from_slice(&width.to_le_bytes());
		bytes.extend_from_slice(&height.to_le_bytes());
		bytes.extend_from_slice(&[0, 0, 0]);
		bytes
	}

	fn image_table(images: &[Vec<u8>]) -> RawTable {
		let mut bytes = b"image,target\n".to_vec();
		for (index, image) in images.iter().enumerate() {
			bytes.extend_from_slice(image);
			bytes.push(b',');
			bytes.extend_from_slice(index.to_string().as_bytes());
			bytes.push(b'\n');
		}
		table(&bytes, u64::try_from(images.len() + 1).unwrap())
	}

	#[test]
	fn no_show_shaped_table_filters_before_split_and_preserves_source_vectors() {
		let raw = table(
			b"PatientId,AppointmentID,Gender,ScheduledDay,AppointmentDay,Age,Neighbourhood,Scholarship,\
			Hipertension,Diabetes,Alcoholism,Handcap,SMS_received,No-show\n\
			29872499824296,5642903,F,2016-04-29T18:38:08Z,2016-04-29T00:00:00Z,62,JARDIM DA PENHA,0,1,0,0,0,0,No\n\
			558997776694438,5642503,M,2016-04-29T16:08:27Z,2016-04-29T00:00:00Z,-1,JARDIM DA PENHA,0,0,0,0,0,0,No\n\
			4262962299951,5642549,F,2016-04-30T16:19:04Z,2016-04-30T00:00:00Z,18,MATA DA PRAIA,0,0,0,0,0,0,Yes\n\
			867951213174,5642828,F,2016-05-01T08:00:00Z,2016-05-01T00:00:00Z,33,PONTAL,1,0,0,0,0,1,No\n\
			8841186448183,5642494,M,2016-05-02T09:00:00Z,2016-05-02T00:00:00Z,44,CENTRO,0,1,1,0,0,1,Yes\n",
			8,
		);
		let request = PreparationRequest::new(["No-show"], fraction())
			.exclude_columns([ColumnPattern::new("*ID*").unwrap()])
			.exclude_rows([RowPredicate::new(
				"Age",
				ComparisonOperator::Less,
				PredicateLiteral::Signed(0),
			)]);
		let prepared = prepare_table(&raw, &request, &CategoricalEncodingModel).unwrap();
		assert_eq!(raw.width(), 14);
		assert_eq!(prepared.vectors().len(), 12);
		assert_eq!(
			prepared
				.vectors()
				.iter()
				.map(PreparedVector::source_index)
				.collect::<Vec<_>>(),
			(2..14).collect::<Vec<_>>()
		);
		assert_eq!(prepared.retained_source_rows(), [0, 2, 3, 4]);
		assert_eq!(prepared.excluded_source_rows(), [1]);
		assert_eq!(prepared.train().source_rows(), [0, 2, 3]);
		assert_eq!(prepared.validation().source_rows(), [4]);
		assert_eq!(
			prepared
				.vectors()
				.iter()
				.filter(|vector| vector.role() == VectorRole::Target)
				.map(PreparedVector::name)
				.collect::<Vec<_>>(),
			[b"No-show".as_slice()]
		);
	}

	#[test]
	fn semantic_encodings_fit_categorical_state_on_train_rows_and_keep_one_vector_each() {
		let raw = table(
			b"integer,float,time,label,rank,prose,target\n\
			-2,1.25,2024-01-02T00:00:00Z,zeta,low,\"first sentence with spaces\",yes\n\
			0,-0.5,2024-01-01T00:00:00Z,alpha,medium,\"second sentence with spaces\",no\n\
			7,2.0,2024-01-03T00:00:00Z,zeta,high,\"third sentence with spaces\",yes\n\
			9,3.5,2024-01-04T00:00:00Z,beta,medium,\"fourth sentence with spaces\",no\n",
			8,
		);
		let prepared = prepare_table(
			&raw,
			&PreparationRequest::new(["target"], TrainFraction::new(1, 2).unwrap()),
			&CategoricalEncodingModel,
		)
		.unwrap();
		assert_eq!(prepared.vectors().len(), raw.width());
		let integer = &prepared.vectors()[0];
		assert_eq!(
			integer.values(),
			&PreparedValues::I32(vec![Some(-2), Some(0), Some(7), Some(9)])
		);
		let float = &prepared.vectors()[1];
		assert_eq!(
			float.values(),
			&PreparedValues::F32Bits(vec![
				Some(1.25f32.to_bits()),
				Some((-0.5f32).to_bits()),
				Some(2.0f32.to_bits()),
				Some(3.5f32.to_bits()),
			])
		);
		let time = &prepared.vectors()[2];
		assert_eq!(
			time.metadata(),
			&VectorMetadata::Temporal {
				origin: TemporalOrigin {
					unix_seconds: 1_704_067_200,
					nanoseconds: 0,
				}
			}
		);
		assert_eq!(
			time.values(),
			&PreparedValues::I32(vec![Some(86_400), Some(0), Some(172_800), Some(259_200)])
		);
		let category = &prepared.vectors()[3];
		assert_eq!(
			category.metadata(),
			&VectorMetadata::Categorical {
				dictionary: vec![b"alpha".to_vec(), b"zeta".to_vec()]
			}
		);
		assert_eq!(
			category.values(),
			&PreparedValues::I32(vec![Some(1), Some(0), Some(1), Some(2)])
		);
		assert_eq!(
			prepared.vectors()[4].values(),
			&PreparedValues::I32(vec![Some(0), Some(1), Some(2), Some(1)])
		);
		let PreparedValues::VariableWidth(prose) = prepared.vectors()[5].values() else {
			panic!("prose must remain variable-width");
		};
		assert_eq!(
			prose.value(1),
			Some(Some(b"second sentence with spaces".as_slice()))
		);
	}

	#[test]
	fn dense_projection_fails_instead_of_casting_imputing_or_flattening_text() {
		let raw = table(
			b"sentence,target\n\"one sentence with words\",1\n\"another sentence with words\",0\n",
			4,
		);
		let prepared = prepare_table(
			&raw,
			&PreparationRequest::new(["target"], TrainFraction::new(1, 2).unwrap()),
			&CategoricalEncodingModel,
		)
		.unwrap();
		assert_eq!(
			prepared
				.fixed_dense_matrix(VectorRole::Feature, PartitionKind::Train)
				.unwrap_err()
				.kind,
			PrepareErrorKind::VariableWidthDenseMatrix
		);
		assert_eq!(
			prepared
				.fixed_dense_matrix(VectorRole::Target, PartitionKind::Train)
				.unwrap(),
			DenseMatrix::I32 {
				rows: 1,
				columns: 1,
				values: vec![1],
			}
		);

		let missing = table(b"x,target\n1,0\n,1\n", 4);
		let prepared = prepare_table(
			&missing,
			&PreparationRequest::new(["target"], TrainFraction::new(1, 2).unwrap()),
			&CategoricalEncodingModel,
		)
		.unwrap();
		assert_eq!(
			prepared
				.fixed_dense_matrix(VectorRole::Feature, PartitionKind::Validation)
				.unwrap_err()
				.kind,
			PrepareErrorKind::MissingDenseValue
		);
	}

	#[test]
	fn dense_projection_combines_only_exact_mixed_numeric_values() {
		let raw = table(b"float,count,target\n1.5,1,0\n2.5,2,1\n", 4);
		let prepared = prepare_table(
			&raw,
			&PreparationRequest::new(["target"], TrainFraction::new(1, 2).unwrap()),
			&CategoricalEncodingModel,
		)
		.unwrap();
		assert_eq!(
			prepared
				.fixed_dense_matrix(VectorRole::Feature, PartitionKind::Train)
				.unwrap(),
			DenseMatrix::F32Bits {
				rows: 1,
				columns: 2,
				values: vec![1.5f32.to_bits(), 1.0f32.to_bits()],
			}
		);

		let lossy = table(b"float,count,target\n1.5,1,0\n2.5,16777217,1\n", 4);
		let prepared = prepare_table(
			&lossy,
			&PreparationRequest::new(["target"], TrainFraction::new(1, 2).unwrap()),
			&CategoricalEncodingModel,
		)
		.unwrap();
		let error = prepared
			.fixed_dense_matrix(VectorRole::Feature, PartitionKind::Validation)
			.unwrap_err();
		assert_eq!(error.kind, PrepareErrorKind::MixedDenseEncoding);
		assert_eq!(error.column.as_deref(), Some(b"count".as_slice()));
		assert_eq!(error.source_row, Some(1));
	}

	#[test]
	fn predicates_are_typed_and_all_comparison_operators_are_exact() {
		let raw = table(b"value,label,target\n-2,ant,0\n0,bee,1\n3,cat,0\n", 5);
		let cases = [
			(
				ComparisonOperator::Equal,
				PredicateLiteral::Signed(0),
				vec![0, 2],
			),
			(
				ComparisonOperator::NotEqual,
				PredicateLiteral::Signed(0),
				vec![1],
			),
			(
				ComparisonOperator::Less,
				PredicateLiteral::Signed(0),
				vec![1, 2],
			),
			(
				ComparisonOperator::LessOrEqual,
				PredicateLiteral::Signed(0),
				vec![2],
			),
			(
				ComparisonOperator::Greater,
				PredicateLiteral::Signed(0),
				vec![0, 1],
			),
			(
				ComparisonOperator::GreaterOrEqual,
				PredicateLiteral::Signed(0),
				vec![0],
			),
		];
		for (operator, literal, expected) in cases {
			let request = PreparationRequest::new(["target"], fraction())
				.exclude_rows([RowPredicate::new("value", operator, literal)]);
			assert_eq!(
				prepare_table(&raw, &request, &CategoricalEncodingModel)
					.unwrap()
					.retained_source_rows(),
				expected
			);
		}
		let text = PreparationRequest::new(["target"], fraction()).exclude_rows([RowPredicate::new(
			"label",
			ComparisonOperator::GreaterOrEqual,
			PredicateLiteral::Text("bee".to_owned()),
		)]);
		assert_eq!(
			prepare_table(&raw, &text, &CategoricalEncodingModel)
				.unwrap()
				.retained_source_rows(),
			[0]
		);

		let typed = table(b"float,count,target\n-1.25,0,0\n0.5,2,1\n3.0,4,0\n", 5);
		let unsigned = PreparationRequest::new(["target"], fraction()).exclude_rows([RowPredicate::new(
			"count",
			ComparisonOperator::Greater,
			PredicateLiteral::Unsigned(1),
		)]);
		assert_eq!(
			prepare_table(&typed, &unsigned, &CategoricalEncodingModel)
				.unwrap()
				.retained_source_rows(),
			[0]
		);
		let float = PreparationRequest::new(["target"], fraction()).exclude_rows([RowPredicate::new(
			"float",
			ComparisonOperator::LessOrEqual,
			PredicateLiteral::f32(0.5),
		)]);
		assert_eq!(
			prepare_table(&typed, &float, &CategoricalEncodingModel)
				.unwrap()
				.retained_source_rows(),
			[2]
		);
	}

	#[test]
	fn selection_and_predicate_errors_fail_closed() {
		let raw = table(b"id,name,target\n1,alpha,0\n2,beta,1\n", 4);
		let unmatched = PreparationRequest::new(["target"], fraction())
			.exclude_columns([ColumnPattern::new("*missing*").unwrap()]);
		assert_eq!(
			prepare_table(&raw, &unmatched, &CategoricalEncodingModel)
				.unwrap_err()
				.kind,
			PrepareErrorKind::UnmatchedColumnPattern
		);
		let target = PreparationRequest::new(["target"], fraction())
			.exclude_columns([ColumnPattern::new("*target*").unwrap()]);
		assert_eq!(
			prepare_table(&raw, &target, &CategoricalEncodingModel)
				.unwrap_err()
				.kind,
			PrepareErrorKind::TargetExcluded
		);
		let wrong_type = PreparationRequest::new(["target"], fraction()).exclude_rows([RowPredicate::new(
			"name",
			ComparisonOperator::Equal,
			PredicateLiteral::Signed(1),
		)]);
		assert_eq!(
			prepare_table(&raw, &wrong_type, &CategoricalEncodingModel)
				.unwrap_err()
				.kind,
			PrepareErrorKind::PredicateTypeMismatch
		);
		let all_rows = PreparationRequest::new(["target"], fraction()).exclude_rows([RowPredicate::new(
			"id",
			ComparisonOperator::Greater,
			PredicateLiteral::Signed(0),
		)]);
		assert_eq!(
			prepare_table(&raw, &all_rows, &CategoricalEncodingModel)
				.unwrap_err()
				.kind,
			PrepareErrorKind::NoRetainedRows
		);
	}

	#[test]
	fn fraction_is_reduced_and_split_uses_exact_integer_arithmetic() {
		assert_eq!(
			TrainFraction::new(8, 10).unwrap(),
			TrainFraction {
				numerator: 4,
				denominator: NonZeroU64::new(5).unwrap(),
			}
		);
		assert_eq!(
			TrainFraction::new(0, 5).unwrap_err().kind,
			PrepareErrorKind::InvalidTrainFraction
		);
		assert_eq!(
			TrainFraction::new(5, 5).unwrap_err().kind,
			PrepareErrorKind::InvalidTrainFraction
		);
		assert_eq!(
			TrainFraction::new(1, 0).unwrap_err().kind,
			PrepareErrorKind::InvalidTrainFraction
		);
		assert_eq!(
			TrainFraction::from_f32(0.8),
			TrainFraction::new(13_421_773, 16_777_216)
		);
		assert_eq!(
			TrainFraction::from_f32(f32::MIN_POSITIVE).unwrap_err().kind,
			PrepareErrorKind::InvalidTrainFraction
		);
	}

	#[test]
	fn temporal_offsets_are_lossless_or_rejected_and_images_remain_variable_width() {
		let first = jpeg(23, 17);
		let second = jpeg(23, 17);
		let mut bytes = b"time,image,target\n2024-01-01T00:00:00.125+01:00,".to_vec();
		bytes.extend_from_slice(&first);
		bytes.extend_from_slice(b",0\n2024-01-01T00:00:01.125+01:00,");
		bytes.extend_from_slice(&second);
		bytes.extend_from_slice(b",1\n");
		let temporal = table(&bytes, 4);
		let prepared = prepare_table(
			&temporal,
			&PreparationRequest::new(["target"], TrainFraction::new(1, 2).unwrap()),
			&CategoricalEncodingModel,
		)
		.unwrap();
		assert_eq!(
			prepared.vectors()[0].metadata(),
			&VectorMetadata::Temporal {
				origin: TemporalOrigin {
					unix_seconds: 1_704_063_600,
					nanoseconds: 125_000_000,
				}
			}
		);
		assert_eq!(
			prepared.vectors()[0].values(),
			&PreparedValues::I32(vec![Some(0), Some(1)])
		);
		assert!(matches!(
			prepared.vectors()[1].values(),
			PreparedValues::VariableWidth(_)
		));
		assert!(matches!(
			prepared.vectors()[1].metadata(),
			VectorMetadata::Image { encoded_variants } if encoded_variants.len() == 1
		));

		let lossy = table(
			b"time,target\n2024-01-01T00:00:00.1Z,0\n2024-01-01T00:00:01.2Z,1\n",
			4,
		);
		assert_eq!(
			prepare_table(
				&lossy,
				&PreparationRequest::new(["target"], TrainFraction::new(1, 2).unwrap()),
				&CategoricalEncodingModel,
			)
			.unwrap_err()
			.kind,
			PrepareErrorKind::EncodingFailure
		);
	}

	#[test]
	fn mixed_image_shapes_remain_encoded_and_clone_into_vector_schema() {
		let images = [jpeg(23, 17), jpeg(29, 19)];
		let raw = image_table(&images);
		let prepared = prepare_table(
			&raw,
			&PreparationRequest::new(["target"], TrainFraction::new(1, 2).unwrap()),
			&CategoricalEncodingModel,
		)
		.unwrap();
		let image = &prepared.vectors()[0];
		let VectorMetadata::Image { encoded_variants } = image.metadata() else {
			panic!("image vector must retain encoded header metadata");
		};
		assert_eq!(encoded_variants.len(), 2);
		assert_eq!(
			encoded_variants
				.iter()
				.map(|variant| (variant.width(), variant.height()))
				.collect::<Vec<_>>(),
			[(23, 17), (29, 19)]
		);
		assert!(encoded_variants.iter().all(|variant| {
			variant.format() == crate::EncodedImageFormat::Jpeg
				&& variant.channels() == Some(3)
				&& variant.color_model() == Some(crate::ImageColorModel::YCbCr)
				&& variant.sample_bits() == Some(8)
				&& variant.value_layout() == crate::ImageValueLayout::EncodedFile
				&& variant.value_range() == crate::ImageValueRange::EncodedBytes
		}));
		let PreparedValues::VariableWidth(values) = image.values() else {
			panic!("encoded image bytes must remain variable-width");
		};
		assert_eq!(values.value(0), Some(Some(images[0].as_slice())));
		assert_eq!(values.value(1), Some(Some(images[1].as_slice())));

		let schema = image.schema();
		let cloned = schema.clone();
		assert_eq!(cloned, schema);
		assert_eq!(schema.source_index(), 0);
		assert_eq!(schema.name(), b"image");
		assert_eq!(schema.role(), VectorRole::Feature);
		assert_eq!(schema.semantic_type(), SemanticType::Image);
		assert_eq!(schema.encoding(), VectorEncoding::Bytes);
		assert_eq!(schema.metadata(), image.metadata());
	}

	#[test]
	fn mixed_image_formats_are_retained_as_distinct_schema_variants() {
		let raw = image_table(&[jpeg(23, 17), gif(31, 19)]);
		let prepared = prepare_table(
			&raw,
			&PreparationRequest::new(["target"], TrainFraction::new(1, 2).unwrap()),
			&CategoricalEncodingModel,
		)
		.unwrap();
		let VectorMetadata::Image { encoded_variants } = prepared.vectors()[0].metadata() else {
			panic!("image vector must retain encoded header metadata");
		};
		assert_eq!(
			encoded_variants
				.iter()
				.map(|variant| (variant.format(), variant.width(), variant.height()))
				.collect::<Vec<_>>(),
			[
				(crate::EncodedImageFormat::Jpeg, 23, 17),
				(crate::EncodedImageFormat::Gif89a, 31, 19),
			]
		);
	}

	#[test]
	fn truncated_image_header_reports_its_source_vector_and_row() {
		let raw = image_table(&[b"\xff\xd8\xff".to_vec(), jpeg(23, 17)]);
		let error = prepare_table(
			&raw,
			&PreparationRequest::new(["target"], TrainFraction::new(1, 2).unwrap()),
			&CategoricalEncodingModel,
		)
		.unwrap_err();
		assert_eq!(error.kind, PrepareErrorKind::EncodingFailure);
		assert_eq!(error.column.as_deref(), Some(b"image".as_slice()));
		assert_eq!(error.source_row, Some(0));
		assert!(error.detail.contains("invalid encoded image header"));
	}

	#[test]
	fn glob_matching_is_ascii_case_insensitive_and_wildcards_are_bounded() {
		assert!(ColumnPattern::new("*ID*").unwrap().matches(b"PatientId"));
		assert!(
			ColumnPattern::new("Appointment??")
				.unwrap()
				.matches(b"AppointmentID")
		);
		assert!(!ColumnPattern::new("*ID").unwrap().matches(b"identity"));
		assert_eq!(
			ColumnPattern::new(Vec::new()).unwrap_err().kind,
			PrepareErrorKind::InvalidColumnPattern
		);
	}
}
