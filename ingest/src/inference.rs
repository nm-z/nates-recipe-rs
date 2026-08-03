use core::fmt; use std::collections::BTreeMap;

use crate::{RawTable, parse_contract_f32, parse_contract_i32, prepare::CategoricalObservation};

/// Exact saved encoding contract for one inference feature.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InferenceFeatureEncoding { NumericI32, NumericF32, CategoricalDictionary { dictionary: Vec<Vec<u8>> }, }

/// Saved identity and encoding of one feature consumed by a model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InferenceFeatureSchema { source_vector: usize, name: Vec<u8>, encoding: InferenceFeatureEncoding, }

impl InferenceFeatureSchema {
	#[must_use]
	pub fn new(source_vector: usize, name: impl Into<Vec<u8>>, encoding: InferenceFeatureEncoding) -> Self { Self {
			source_vector, name: name.into(), encoding, } }

	#[must_use]
	pub const fn source_vector(&self) -> usize { self.source_vector }

	#[must_use]
	pub fn name(&self) -> &[u8] { &self.name }

	#[must_use]
	pub const fn encoding(&self) -> &InferenceFeatureEncoding { &self.encoding } }

/// Stable location inside a schema-driven inference input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InferenceDataPath { feature: usize, source_vector: usize, column: Vec<u8>, source_row: Option<usize>, }

impl InferenceDataPath {
	#[must_use]
	pub const fn feature(&self) -> usize { self.feature }

	#[must_use]
	pub const fn source_vector(&self) -> usize { self.source_vector }

	#[must_use]
	pub fn column(&self) -> &[u8] { &self.column }

	#[must_use]
	pub const fn source_row(&self) -> Option<usize> { self.source_row }

	fn new(feature: usize, schema: &InferenceFeatureSchema) -> Self { Self { feature, source_vector: schema.source_vector,
			column: schema.name.clone(), source_row: None, } }

	fn for_row(mut self, source_row: usize) -> Self { self.source_row = Some(source_row); self } }

impl fmt::Display for InferenceDataPath {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!( formatter,
			"inference.feature[{}].source-vector[{}].column[{:?}]",
			self.feature, self.source_vector, String::from_utf8_lossy(&self.column) )?;
		if let Some(source_row) = self.source_row {
			write!(formatter, ".source-row[{source_row}]")?;
		}
		Ok(()) } }

/// Stable class of schema-driven inference input failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum InferencePrepareErrorKind { EmptyFeatureSchema, InvalidFeatureSchema, MissingRequiredFeature,
	AmbiguousRequiredFeature, MissingValue, InvalidValue, ArithmeticOverflow, }

/// Path-addressed failure to apply a saved feature schema to new rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InferencePrepareError { kind: InferencePrepareErrorKind, path: Option<InferenceDataPath>, detail: String, }

impl InferencePrepareError {
	#[must_use]
	pub const fn kind(&self) -> InferencePrepareErrorKind { self.kind }

	#[must_use]
	pub const fn path(&self) -> Option<&InferenceDataPath> { self.path.as_ref() }

	#[must_use]
	pub fn detail(&self) -> &str { &self.detail }

	fn new(kind: InferencePrepareErrorKind, detail: impl Into<String>) -> Self { Self { kind, path: None,
			detail: detail.into(), } }

	fn at(mut self, path: InferenceDataPath) -> Self { self.path = Some(path); self } }

impl fmt::Display for InferencePrepareError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(formatter, "{:?}: {}", self.kind, self.detail)?;
		if let Some(path) = &self.path {
			write!(formatter, " [{path}]")?;
		}
		Ok(()) } }

impl std::error::Error for InferencePrepareError {}

pub type InferencePrepareResult<T> = Result<T, InferencePrepareError>;

/// Typed, unnormalized values parsed under one saved feature contract.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PreparedInferenceValues { I32(Vec<i32>), F32Bits(Vec<u32>), }

impl PreparedInferenceValues {
	#[must_use]
	pub fn len(&self) -> usize { match self { Self::I32(values) => values.len(), Self::F32Bits(values) => values.len(), } }

	#[must_use]
	pub fn is_empty(&self) -> bool { self.len() == 0 } }

/// One required model feature resolved from a possibly reordered source table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedInferenceFeature { schema: InferenceFeatureSchema, source_column: usize,
	values: PreparedInferenceValues, categorical_observations: Option<Vec<CategoricalObservation>>, }

impl PreparedInferenceFeature {
	#[must_use]
	pub const fn schema(&self) -> &InferenceFeatureSchema { &self.schema }

	#[must_use]
	pub const fn source_column(&self) -> usize { self.source_column }

	#[must_use]
	pub const fn values(&self) -> &PreparedInferenceValues { &self.values }

	/// Typed categorical observations in inference-row order.
	///
	/// Existing graph consumers retain calculation-facing i32 codes in
	/// [`Self::values`]. This parallel route preserves the distinction between a
	/// missing source value and an unseen nonempty label, including its bytes.
	#[must_use]
	pub fn categorical_observations(&self) -> Option<&[CategoricalObservation]> { self.categorical_observations.as_deref() }
}

/// All inference rows parsed under the saved model feature schema.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedInferenceDataset { rows: usize, features: Vec<PreparedInferenceFeature>, }

impl PreparedInferenceDataset {
	#[must_use]
	pub const fn rows(&self) -> usize { self.rows }

	#[must_use]
	pub fn features(&self) -> &[PreparedInferenceFeature] { &self.features } }

/// Parse only the saved model features from a new table.
///
/// Feature identity is matched by exact column-name bytes, so source columns
/// may be reordered and unrelated columns are ignored. No semantic inference,
/// dictionary fitting, target selection, row filtering, or train/evaluation
/// split occurs. Missing numeric values fail closed. Existing graph consumers
/// receive the checkpoint-v5 reserved code for categorical missing/unseen
/// values, while [`PreparedInferenceFeature::categorical_observations`] records
/// their distinct typed routes and preserves every unseen label.
///
/// # Errors
///
/// Returns a typed, path-addressed error for an invalid saved schema, a missing
/// or duplicate required source column, an invalid value, or arithmetic
/// overflow.
pub fn prepare_inference_table( table: &RawTable, schema: &[InferenceFeatureSchema],
) -> InferencePrepareResult<PreparedInferenceDataset> { if schema.is_empty() { return Err(InferencePrepareError::new(
			InferencePrepareErrorKind::EmptyFeatureSchema,
			"saved model schema contains no features",
		)); }
	let mut saved_names = BTreeMap::<&[u8], usize>::new(); let mut saved_sources = BTreeMap::<usize, usize>::new();
	for (feature, feature_schema) in schema.iter().enumerate() {
		let path = InferenceDataPath::new(feature, feature_schema); validate_schema(feature_schema, &path)?;
		if let Some(first) = saved_names.insert(feature_schema.name(), feature) { return Err(InferencePrepareError::new(
				InferencePrepareErrorKind::InvalidFeatureSchema,
				format!("saved feature name duplicates feature {first}"),
			) .at(path)); }
		if let Some(first) = saved_sources.insert(feature_schema.source_vector(), feature) {
			return Err(InferencePrepareError::new( InferencePrepareErrorKind::InvalidFeatureSchema,
				format!("saved source-vector identity duplicates feature {first}"),
			) .at(path)); } }
	let columns = table.headers().iter().enumerate().fold( BTreeMap::<&[u8], Vec<usize>>::new(),
		|mut columns, (index, name)| { columns.entry(name).or_default().push(index); columns }, ); let features = schema
		.iter() .enumerate() .map(|(feature, schema)| { let path = InferenceDataPath::new(feature, schema);
			let matches = columns.get(schema.name()).ok_or_else(|| { InferencePrepareError::new(
					InferencePrepareErrorKind::MissingRequiredFeature,
					"required model feature is absent from the inference source",
				) .at(path.clone()) })?; if matches.len() != 1 { return Err(InferencePrepareError::new(
					InferencePrepareErrorKind::AmbiguousRequiredFeature, format!(
						"required model feature appears {} times in the inference source",
						matches.len() ), ) .at(path)); }
			let source_column = matches[0];
			let (values, categorical_observations) = encode_feature(table, source_column, schema, feature)?;
			if values.len() != table.rows().len() || categorical_observations .as_ref()
					.is_some_and(|observations| observations.len() != table.rows().len())
			{ return Err(InferencePrepareError::new( InferencePrepareErrorKind::ArithmeticOverflow,
					"schema application did not preserve inference-row length",
				) .at(path)); }
			Ok(PreparedInferenceFeature { schema: schema.clone(), source_column, values, categorical_observations, }) })
		.collect::<InferencePrepareResult<Vec<_>>>()?; Ok(PreparedInferenceDataset { rows: table.rows().len(), features, }) }

fn validate_schema(schema: &InferenceFeatureSchema, path: &InferenceDataPath) -> InferencePrepareResult<()> {
	if schema.name.is_empty() { return Err(InferencePrepareError::new( InferencePrepareErrorKind::InvalidFeatureSchema,
			"saved feature name is empty",
		) .at(path.clone())); }
	let InferenceFeatureEncoding::CategoricalDictionary { dictionary } = &schema.encoding else { return Ok(()); };
	if dictionary.iter().any(Vec::is_empty) || dictionary .windows(2)
			.any(|labels| labels[0].as_slice() >= labels[1].as_slice())
	{ return Err(InferencePrepareError::new( InferencePrepareErrorKind::InvalidFeatureSchema,
			"saved categorical dictionary is not canonical ascending nonempty byte labels",
		) .at(path.clone())); }
	i32::try_from(dictionary.len()).map_err(|error| { InferencePrepareError::new(
			InferencePrepareErrorKind::InvalidFeatureSchema,
			format!("saved categorical reserved code exceeds i32: {error}"),
		) .at(path.clone()) })?; Ok(()) }

fn encode_feature( table: &RawTable, source_column: usize, schema: &InferenceFeatureSchema, feature: usize,
) -> InferencePrepareResult<(PreparedInferenceValues, Option<Vec<CategoricalObservation>>)> { match &schema.encoding {
		InferenceFeatureEncoding::NumericI32 => { table.rows() .iter() .enumerate() .map(|(source_row, row)| {
					let value = source_value(row, source_column, schema, feature, source_row)?; if value.is_empty() {
						return Err(missing_value(schema, feature, source_row)); }
					let text = core::str::from_utf8(value) .map_err(|error| invalid_value(schema, feature, source_row, error))?;
					parse_contract_i32(text) .map(|value| value.value())
						.map_err(|error| invalid_value(schema, feature, source_row, error)) })
				.collect::<InferencePrepareResult<Vec<_>>>() .map(|values| (PreparedInferenceValues::I32(values), None)) }
		InferenceFeatureEncoding::NumericF32 => { table.rows() .iter() .enumerate() .map(|(source_row, row)| {
					let value = source_value(row, source_column, schema, feature, source_row)?; if value.is_empty() {
						return Err(missing_value(schema, feature, source_row)); }
					let text = core::str::from_utf8(value) .map_err(|error| invalid_value(schema, feature, source_row, error))?;
					parse_contract_f32(text) .map(|value| value.bits())
						.map_err(|error| invalid_value(schema, feature, source_row, error)) })
				.collect::<InferencePrepareResult<Vec<_>>>() .map(|values| (PreparedInferenceValues::F32Bits(values), None)) }
		InferenceFeatureEncoding::CategoricalDictionary { dictionary } => { let codes = dictionary .iter() .enumerate()
				.map(|(code, label)| { i32::try_from(code) .map(|code| (label.as_slice(), code))
						.map_err(|error| invalid_value(schema, feature, 0, error)) })
				.collect::<InferencePrepareResult<BTreeMap<_, _>>>()?; let reserved =
				i32::try_from(dictionary.len()).map_err(|error| invalid_value(schema, feature, 0, error))?; let encoded = table
				.rows() .iter() .enumerate() .map(|(source_row, row)| {
					let value = source_value(row, source_column, schema, feature, source_row)?; if value.is_empty() {
						Ok((reserved, CategoricalObservation::Missing)) } else if let Some(code) = codes.get(value).copied() {
						Ok((code, CategoricalObservation::Known { code })) } else { Ok((reserved, CategoricalObservation::Unseen {
							label: value.to_vec(), })) } }) .collect::<InferencePrepareResult<Vec<_>>>()?;
			let (values, observations): (Vec<i32>, Vec<CategoricalObservation>) = encoded.into_iter().unzip();
			validate_categorical_alignment(schema, feature, reserved, &values, &observations)?;
			Ok((PreparedInferenceValues::I32(values), Some(observations))) } } }

fn validate_categorical_alignment( schema: &InferenceFeatureSchema, feature: usize, reserved: i32, codes: &[i32],
	observations: &[CategoricalObservation], ) -> InferencePrepareResult<()> { if codes.len() != observations.len() {
		return Err(invalid_value( schema, feature, 0,
			"categorical calculation codes and observation routes have different lengths",
		)); }
	for (source_row, (code, observation)) in codes.iter().zip(observations).enumerate() { let aligned = match observation {
			CategoricalObservation::Known { code: known } => *code == *known && *known >= 0 && *known < reserved,
			CategoricalObservation::Missing => *code == reserved,
			CategoricalObservation::Unseen { label } => !label.is_empty() && *code == reserved, }; if !aligned {
			return Err(invalid_value( schema, feature, source_row,
				"categorical calculation code does not match its typed observation route",
			)); } }
	Ok(()) }

fn source_value<'a>(
	row: &'a [Vec<u8>],
	source_column: usize, schema: &InferenceFeatureSchema, feature: usize, source_row: usize,
) -> InferencePrepareResult<&'a [u8]> {
	row.get(source_column).map(Vec::as_slice).ok_or_else(|| { InferencePrepareError::new(
			InferencePrepareErrorKind::InvalidValue,
			"source row is missing its resolved feature column",
		) .at(InferenceDataPath::new(feature, schema).for_row(source_row)) }) }

fn missing_value(schema: &InferenceFeatureSchema, feature: usize, source_row: usize) -> InferencePrepareError {
	InferencePrepareError::new( InferencePrepareErrorKind::MissingValue,
		"required numeric feature value is missing",
	) .at(InferenceDataPath::new(feature, schema).for_row(source_row)) }

fn invalid_value( schema: &InferenceFeatureSchema, feature: usize, source_row: usize, error: impl fmt::Display,
) -> InferencePrepareError { InferencePrepareError::new( InferencePrepareErrorKind::InvalidValue,
		format!("value violates the saved feature encoding: {error}"),
	) .at(InferenceDataPath::new(feature, schema).for_row(source_row)) }
