use core::fmt;

use recipe_ingest::{
	CategoricalEncodingModel, ColumnPattern, ComparisonOperator as IngestComparisonOperator, DatasetSourceError,
	DistilledDataset, IngestError, IngestLimits, PredicateLiteral, PreparationRequest, PrepareError, PreparedDataset,
	RawTable, RowPredicate, SemanticError, TrainFraction, distill_datasets, prepare_inferred_table, select_table,
};

use crate::api::{ComparisonOperator, Condition, ConditionValue, Data, DeclarationError};

/// Default upper bound for one automatically loaded data source.
pub const DEFAULT_DATA_SOURCE_BYTES: u64 = 1 << 30;
/// Default upper bound for framed records, including the header.
pub const DEFAULT_DATA_RECORDS: u64 = 10_000_000;
/// Default upper bound for vectors in one filesystem table.
pub const DEFAULT_DATA_FIELDS_PER_RECORD: u32 = 1 << 14;
/// Default upper bound for one source field.
pub const DEFAULT_DATA_FIELD_BYTES: u64 = 16 << 20;

/// Failure to translate and prepare a public [`Data`] declaration.
#[derive(Debug)]
#[non_exhaustive]
pub enum DataPreparationError {
	Declaration(DeclarationError),
	MissingTargets,
	MissingSplit,
	FloatPredicateOutsideF32 { column: String, value: f64 },
	Ingest(IngestError),
	Source(DatasetSourceError),
	Semantic(SemanticError),
	Prepare(PrepareError),
}

impl fmt::Display for DataPreparationError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Declaration(error) => write!(formatter, "invalid data declaration: {error}"),
			Self::MissingTargets => write!(
				formatter,
				"dataset preparation requires at least one target vector"
			),
			Self::MissingSplit => write!(
				formatter,
				"dataset preparation requires an explicit train split"
			),
			Self::FloatPredicateOutsideF32 { column, value } => write!(
				formatter,
				"predicate for column {column:?} cannot be represented as a finite, non-underflowing f32: {value}"
			),
			Self::Ingest(error) => write!(formatter, "load data source: {error}"),
			Self::Source(error) => write!(formatter, "distill data source: {error}"),
			Self::Semantic(error) => write!(formatter, "infer data vectors: {error}"),
			Self::Prepare(error) => write!(formatter, "prepare data source: {error}"),
		}
	}
}

impl std::error::Error for DataPreparationError {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match self {
			Self::Declaration(error) => Some(error),
			Self::Ingest(error) => Some(error),
			Self::Source(error) => Some(error),
			Self::Semantic(error) => Some(error),
			Self::Prepare(error) => Some(error),
			Self::MissingTargets | Self::MissingSplit | Self::FloatPredicateOutsideF32 { .. } => None,
		}
	}
}

pub type DataPreparationResult<T> = Result<T, DataPreparationError>;

/// Load and prepare one public data declaration with finite default bounds.
///
/// Files, recursively discovered directories, and ZIP containers are
/// distilled into one rectangular list of vectors. Known container structure
/// is retained as ordinary semantic vectors; ambiguous textual vectors are passed
/// through the configured semantic model. Preparation performs no imputation,
/// normalization, or lossy scalar conversion.
///
/// # Errors
///
/// Fails before reading for an invalid or unsupported declaration. Source
/// framing, semantic inference, selection, filtering, lossless encoding, and
/// splitting errors are returned without a partial dataset.
pub fn prepare_data(data: &Data) -> DataPreparationResult<PreparedDataset> {
	let limits = default_data_limits()?;
	prepare_data_with_limits(data, limits)
}

/// Distill one public data declaration without applying training-only target,
/// split, exclusion, or normalization policy.
///
/// This is the shared bounded source boundary for target-free inference. It
/// preserves declared source order and all filesystem/container semantics.
pub fn distill_data(data: &Data) -> DataPreparationResult<DistilledDataset> {
	distill_data_with_limits(data, default_data_limits()?)
}

/// Distill one public data declaration under caller-fixed aggregate bounds.
pub fn distill_data_with_limits(data: &Data, limits: IngestLimits) -> DataPreparationResult<DistilledDataset> {
	data.validate().map_err(DataPreparationError::Declaration)?;
	distill_datasets(data.sources().iter().map(std::path::Path::new), limits).map_err(DataPreparationError::Source)
}

/// Apply row and column exclusions without target selection, semantic fitting,
/// splitting, or normalization.
///
/// This is the target-free selection boundary used by inference. The saved
/// model schema remains authoritative after selection.
pub fn select_target_free_data(data: &Data, distilled: &DistilledDataset) -> DataPreparationResult<RawTable> {
	let excluded_columns = data
		.exclusions()
		.iter()
		.map(|pattern| ColumnPattern::new(pattern.as_bytes().to_vec()).map_err(DataPreparationError::Prepare))
		.collect::<DataPreparationResult<Vec<_>>>()?;
	let excluded_rows = data
		.condition_exclusions()
		.iter()
		.map(map_condition)
		.collect::<DataPreparationResult<Vec<_>>>()?;
	select_table(distilled.table(), excluded_columns, excluded_rows).map_err(DataPreparationError::Prepare)
}

/// Load and prepare one public data declaration with caller-fixed bounds.
///
/// This is the preparation-time filesystem boundary. Runtime code receives
/// only the returned [`PreparedDataset`].
///
/// # Errors
///
/// Returns [`DataPreparationError`] under the same fail-closed contract as
/// [`prepare_data`].
pub fn prepare_data_with_limits(data: &Data, limits: IngestLimits) -> DataPreparationResult<PreparedDataset> {
	data.validate().map_err(DataPreparationError::Declaration)?;
	if data.targets().is_empty() {
		return Err(DataPreparationError::MissingTargets);
	}
	let split = data
		.split_fraction()
		.ok_or(DataPreparationError::MissingSplit)?;
	let train_fraction = TrainFraction::from_f32(split).map_err(DataPreparationError::Prepare)?;
	let excluded_columns = data
		.exclusions()
		.iter()
		.map(|pattern| ColumnPattern::new(pattern.as_bytes().to_vec()).map_err(DataPreparationError::Prepare))
		.collect::<DataPreparationResult<Vec<_>>>()?;
	let excluded_rows = data
		.condition_exclusions()
		.iter()
		.map(map_condition)
		.collect::<DataPreparationResult<Vec<_>>>()?;
	let request = PreparationRequest::new(
		data.targets()
			.iter()
			.map(|target| target.as_bytes().to_vec()),
		train_fraction,
	)
	.exclude_columns(excluded_columns)
	.exclude_rows(excluded_rows);
	let distilled = distill_data_with_limits(data, limits)?;
	let inferred = distilled
		.infer_vectors(&CategoricalEncodingModel)
		.map_err(DataPreparationError::Semantic)?;
	prepare_inferred_table(distilled.table(), &inferred, &request).map_err(DataPreparationError::Prepare)
}

fn default_data_limits() -> DataPreparationResult<IngestLimits> {
	IngestLimits::new(
		DEFAULT_DATA_SOURCE_BYTES,
		DEFAULT_DATA_RECORDS,
		DEFAULT_DATA_FIELDS_PER_RECORD,
		DEFAULT_DATA_FIELD_BYTES,
	)
	.map_err(DataPreparationError::Ingest)
}

fn map_condition(condition: &Condition) -> DataPreparationResult<RowPredicate> {
	let operator = match condition.operator() {
		ComparisonOperator::Equal => IngestComparisonOperator::Equal,
		ComparisonOperator::NotEqual => IngestComparisonOperator::NotEqual,
		ComparisonOperator::Less => IngestComparisonOperator::Less,
		ComparisonOperator::LessOrEqual => IngestComparisonOperator::LessOrEqual,
		ComparisonOperator::Greater => IngestComparisonOperator::Greater,
		ComparisonOperator::GreaterOrEqual => IngestComparisonOperator::GreaterOrEqual,
	};
	let literal = match condition.value() {
		ConditionValue::Signed(value) => PredicateLiteral::Signed(*value),
		ConditionValue::Unsigned(value) => PredicateLiteral::Unsigned(*value),
		ConditionValue::FloatBits(bits) => {
			let value = f64::from_bits(*bits);
			let narrowed = value as f32;
			if !narrowed.is_finite() || value != 0.0 && narrowed == 0.0 {
				return Err(DataPreparationError::FloatPredicateOutsideF32 {
					column: condition.column().to_owned(),
					value,
				});
			}
			PredicateLiteral::f32(narrowed)
		}
		ConditionValue::Boolean(value) => PredicateLiteral::Text(value.to_string()),
		ConditionValue::Text(value) => PredicateLiteral::Text(value.clone()),
	};
	Ok(RowPredicate::new(
		condition.column().as_bytes().to_vec(),
		operator,
		literal,
	))
}
