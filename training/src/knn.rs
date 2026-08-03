use alloc::collections::{BTreeMap, BTreeSet}; use core::{fmt, num::NonZeroU64};

use recipe_ingest::{
	DenseMatrix, PartitionKind, PreparedDataset, PreparedVector, SemanticType, VectorEncoding, VectorRole, };
use recipe_ops::KnnOutputSpec;

use crate::{ CheckpointArtifactVector, CompiledFeatureSpan, TrainingCompileError, TrainingCompileErrorKind,
	TrainingCompileResult, model::{DenseFeaturePlan, lower_dense_features}, };

/// Exact semantic value represented by one calculation-facing KNN class code.
///
/// Numeric targets are reduced as f32 means and therefore do not use this
/// dictionary. Every nonnumeric target is reduced as a mode over deterministic
/// int32 codes, then decoded through one of these exact values.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct KnnLabelValue {
	/// Representation selected for this exact label.
	kind: KnnLabelValueKind,
	/// Int32 payload when `kind` is `I32`.
	int32: i32,
	/// Byte-string payload when `kind` is `Bytes`.
	bytes: Vec<u8>, }

/// Stored payload representation for one exact KNN label.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum KnnLabelValueKind {
	/// One exact int32 label.
	I32,
	/// One exact byte-string label.
	Bytes, }

impl KnnLabelValue {
	/// Construct one exact int32 label.
	#[must_use]
	#[inline]
	pub const fn i32(value: i32) -> Self { return Self { kind: KnnLabelValueKind::I32, int32: value, bytes: Vec::new(), };
	}

	/// Construct one exact byte-string label.
	#[must_use]
	#[inline]
	pub const fn bytes(value: Vec<u8>) -> Self { return Self { kind: KnnLabelValueKind::Bytes, int32: 0, bytes: value, }; }

	/// Return the int32 value when this is an int32 label.
	#[must_use]
	#[inline]
	pub const fn as_i32(&self) -> Option<i32> { return match self.kind { KnnLabelValueKind::I32 => Some(self.int32),
			KnnLabelValueKind::Bytes => None, }; }

	/// Return the byte string when this is a byte-string label.
	#[must_use]
	#[inline]
	pub fn as_bytes(&self) -> Option<&[u8]> { return match self.kind { KnnLabelValueKind::I32 => None,
			KnnLabelValueKind::Bytes => Some(&self.bytes), }; } }

/// Calculation-facing reference values for one declared target.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnnReferenceValues {
	/// Aggregation representation selected for this output.
	kind: KnnReferenceValueKind,
	/// Exact numeric reference values when `kind` is numeric.
	numeric_f32_bits: Vec<u32>,
	/// Calculation-facing class codes when `kind` is discrete.
	codes: Vec<i32>,
	/// Exact class labels when `kind` is discrete.
	labels: Vec<KnnLabelValue>, }

/// Stored aggregation representation for one KNN output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KnnReferenceValueKind {
	/// Finite numeric values represented by exact f32 bits.
	NumericF32Bits,
	/// Discrete int32 codes paired with an exact label dictionary.
	DiscreteI32, }

impl KnnReferenceValues {
	/// Construct numeric reference values from exact finite f32 bits.
	#[inline]
	pub(super) const fn from_numeric_f32_bits(bits: Vec<u32>) -> Self { return Self {
			kind: KnnReferenceValueKind::NumericF32Bits, numeric_f32_bits: bits, codes: Vec::new(), labels: Vec::new(), }; }

	/// Construct discrete reference values and their exact label dictionary.
	#[inline]
	pub(super) const fn from_discrete_i32(codes: Vec<i32>, labels: Vec<KnnLabelValue>) -> Self { return Self {
			kind: KnnReferenceValueKind::DiscreteI32, numeric_f32_bits: Vec::new(), codes, labels, }; }

	/// Return whether these reference values use numeric aggregation.
	#[inline]
	pub(super) const fn is_numeric(&self) -> bool { return matches!(self.kind, KnnReferenceValueKind::NumericF32Bits); }

	/// Return mutable numeric storage after the caller selects numeric aggregation.
	#[inline]
	pub(super) const fn numeric_f32_bits_mut(&mut self) -> &mut Vec<u32> { return &mut self.numeric_f32_bits; }

	/// Consume and return numeric storage after the caller selects numeric aggregation.
	#[inline]
	pub(super) fn into_numeric_f32_bits(self) -> Vec<u32> { return self.numeric_f32_bits; }

	/// Return mutable discrete storage after the caller selects discrete aggregation.
	#[inline]
	pub(super) const fn discrete_i32_mut(&mut self) -> (&mut Vec<i32>, &mut Vec<KnnLabelValue>) {
		return (&mut self.codes, &mut self.labels); }

	/// Consume and return discrete storage after the caller selects discrete aggregation.
	#[inline]
	pub(super) fn into_discrete_i32(self) -> (Vec<i32>, Vec<KnnLabelValue>) { return (self.codes, self.labels); }

	/// Return exact finite f32 bit patterns for a numeric output.
	#[must_use]
	#[inline]
	pub fn numeric_f32_bits(&self) -> Option<&[u32]> { return match self.kind {
			KnnReferenceValueKind::NumericF32Bits => Some(&self.numeric_f32_bits), KnnReferenceValueKind::DiscreteI32 => None, };
	}

	/// Return numeric storage after the caller selects numeric aggregation.
	#[inline]
	pub(super) fn numeric_f32_bits_storage(&self) -> &[u32] { return &self.numeric_f32_bits; }


	/// Return discrete storage after the caller selects discrete aggregation.
	#[inline]
	pub(super) fn discrete_i32_storage(&self) -> (&[i32], &[KnnLabelValue]) { return (&self.codes, &self.labels); } }

/// One independently reduced KNN output in target declaration order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnnReferenceOutput { pub schema: CheckpointArtifactVector, pub known: Vec<i32>, pub known_references: u64,
	pub values: KnnReferenceValues, }

impl KnnReferenceOutput {
	#[must_use]
	#[inline]
	pub const fn schema(&self) -> &CheckpointArtifactVector { return &self.schema; }

	#[must_use]
	#[inline]
	pub fn known(&self) -> &[i32] { return &self.known; }

	#[must_use]
	#[inline]
	pub const fn known_references(&self) -> u64 { return self.known_references; }

	#[must_use]
	#[inline]
	pub const fn values(&self) -> &KnnReferenceValues { return &self.values; }

	#[must_use]
	#[inline]
	/// # Panics
	///
	/// Panics only if an internally constructed categorical label dictionary
	/// violates the validated `i32` class-count bound.
	pub fn operation_spec(&self) -> KnnOutputSpec { match self.values.kind {
			KnnReferenceValueKind::NumericF32Bits => return KnnOutputSpec::Numeric { known_references: self.known_references, },
			KnnReferenceValueKind::DiscreteI32 => { let Ok(classes) = u64::try_from(self.values.labels.len()) else {
					unreachable!("validated KNN label count fits u64");
				}; return KnnOutputSpec::Categorical { known_references: self.known_references, classes, }; } } }

	/// Decode one discrete prediction. Numeric outputs have no class decoder.
	#[must_use]
	#[inline]
	pub fn decode_class(&self, code: i32) -> Option<&KnnLabelValue> {
		if self.values.kind != KnnReferenceValueKind::DiscreteI32 { return None; }
		let index = usize::try_from(code).ok()?; return self.values.labels.get(index); } }

/// Loss-independent, immutable KNN state prepared from the exact training
/// partition.
///
/// Features use the same typed scalar/one-hot lowering as dense execution but
/// remain unnormalized here. Numerical normalization and its fitted state
/// belong to the eventual execution/checkpoint boundary. Reference rows retain
/// prepared order, which is the deterministic distance-tie order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnnReferenceSet { pub neighbors: NonZeroU64, pub vectors: Vec<CheckpointArtifactVector>,
	pub feature_spans: Vec<CompiledFeatureSpan>, pub normalization_mask: Option<Vec<u32>>,
	pub reference_source_rows: Vec<usize>, pub reference_rows: usize, pub feature_width: usize,
	pub reference_feature_bits: Vec<u32>, pub outputs: Vec<KnnReferenceOutput>, }

impl KnnReferenceSet {
	#[must_use]
	#[inline]
	pub const fn neighbors(&self) -> NonZeroU64 { return self.neighbors; }

	#[must_use]
	#[inline]
	pub fn vectors(&self) -> &[CheckpointArtifactVector] { return &self.vectors; }

	#[must_use]
	#[inline]
	pub fn feature_spans(&self) -> &[CompiledFeatureSpan] { return &self.feature_spans; }

	#[must_use]
	#[inline]
	pub fn normalization_mask(&self) -> Option<&[u32]> { return self.normalization_mask.as_deref(); }

	#[must_use]
	#[inline]
	pub fn reference_source_rows(&self) -> &[usize] { return &self.reference_source_rows; }

	#[must_use]
	#[inline]
	pub const fn reference_rows(&self) -> usize { return self.reference_rows; }

	#[must_use]
	#[inline]
	pub const fn feature_width(&self) -> usize { return self.feature_width; }

	#[must_use]
	#[inline]
	pub fn reference_feature_bits(&self) -> &[u32] { return &self.reference_feature_bits; }

	#[must_use]
	#[inline]
	pub fn outputs(&self) -> &[KnnReferenceOutput] { return &self.outputs; }

	#[must_use]
	#[inline]
	pub fn operation_specs(&self) -> Vec<KnnOutputSpec> { return self.outputs .iter()
			.map(KnnReferenceOutput::operation_spec) .collect(); } }

/// Prepare the immutable reference state consumed by one public
/// `.knn(neighbors)` declaration.
///
/// Exactly one output is emitted for every declared target, in declaration
/// order. Numeric outputs use a uniform mean. Every nonnumeric semantic type
/// uses a uniform mode over an exact deterministic label dictionary. Missing
/// references are retained in row alignment but excluded independently for
/// each output.
///
/// # Errors
///
/// Returns an error when the training partition or targets are empty, feature
/// lowering fails, target declarations are inconsistent, or a target cannot be
/// represented by the KNN reference format.
#[inline]
pub fn prepare_knn_reference_set( dataset: &PreparedDataset, neighbors: NonZeroU64,
) -> TrainingCompileResult<KnnReferenceSet> { if dataset.train().is_empty() { return Err(knn_error(
			TrainingCompileErrorKind::EmptyDataset,
			"KNN requires at least one prepared training reference row",
		)); }
	if dataset.target_source_indices().is_empty() { return Err(knn_error( TrainingCompileErrorKind::InvalidTargetMatrix,
			"KNN requires at least one declared target",
		)); }

	let feature_plan = DenseFeaturePlan::from_prepared(dataset)?;
	let lowered = lower_dense_features(dataset, &feature_plan, PartitionKind::Train)?;
	let (reference_rows, feature_width, reference_feature_bits) = match lowered {
		DenseMatrix::I32 { rows, columns, values } => { let bits = values .into_iter() .enumerate() .map(|(index, value)| {
					let Some(converted) = exact_i32_as_f32(value) else { return Err(knn_error(
							TrainingCompileErrorKind::InvalidFeatureMatrix, format!(
								"KNN feature element {index} int32 value {value} is not exactly representable as f32"
							), )); }; return Ok(converted.to_bits()); }) .collect::<TrainingCompileResult<Vec<_>>>()?; (rows, columns, bits)
		}
		DenseMatrix::F32Bits { rows, columns, values } => { for (index, bits) in values.iter().copied().enumerate() {
				if !f32::from_bits(bits).is_finite() { return Err(knn_error( TrainingCompileErrorKind::InvalidFeatureMatrix,
						format!("KNN feature element {index} has non-finite f32 bits {bits:#010x}"),
					)); } }
			(rows, columns, values) } }; let targets = dataset .vectors() .iter()
		.filter(|vector| return vector.role() == VectorRole::Target) .map(|vector| return (vector.source_index(), vector))
		.collect::<BTreeMap<_, _>>(); let mut seen_targets = BTreeSet::new();
	let mut outputs = Vec::with_capacity(dataset.target_source_indices().len());
	for source_index in dataset.target_source_indices().iter().copied() { if !seen_targets.insert(source_index) {
			return Err(knn_error( TrainingCompileErrorKind::InvalidTargetMatrix,
				format!("KNN target source index {source_index} is declared more than once"),
			)); }
		let target = targets.get(&source_index).copied().ok_or_else(|| { return knn_error(
				TrainingCompileErrorKind::InvalidTargetMatrix,
				format!("KNN declared target source index {source_index} is absent from prepared vectors"),
			); })?; let incompatible_target = || { return knn_target_error( target, 0, format!(
					"incompatible prepared semantic tuple {:?}/{:?}/{:?}",
					target.semantic_type(), target.encoding(), target.metadata(), ), ); }; let prepare_variable_output = || {
			let values = target.values().variable_width().ok_or_else(&incompatible_target)?; return prepare_remapped_labels(
				dataset, target, |position, source_row| { return values .value(position) .ok_or_else(|| { return knn_target_error(
								target, source_row,
								"value is absent from variable-width storage",
							); }) .map(|value| { return value.map(<[u8]>::to_vec); }); }, KnnLabelValue::bytes, ); };
		let output = match (target.semantic_type(), target.encoding()) {
			(SemanticType::Numeric, VectorEncoding::I32) if target.metadata().is_none() => {
				let values = target.values().i32_values().ok_or_else(&incompatible_target)?;
				prepare_numeric_output(dataset, target, values, |value, source_row| {
					let Some(converted) = exact_i32_as_f32(value) else { return Err(knn_target_error( target, source_row,
							format!("int32 value {value} is not exactly representable as f32"),
						)); }; return Ok(converted.to_bits()); }) }
			(SemanticType::Numeric, VectorEncoding::F32) if target.metadata().is_none() => {
				let values = target.values().f32_bits().ok_or_else(&incompatible_target)?;
				prepare_numeric_output(dataset, target, values, |bits, source_row| { if !f32::from_bits(bits).is_finite() {
						return Err(knn_target_error( target, source_row,
							format!("value has non-finite f32 bits {bits:#010x}"),
						)); }
					return Ok(bits); }) }
			(SemanticType::Categorical, VectorEncoding::DictionaryI32) => { let dictionary = target .metadata()
					.categorical_dictionary() .ok_or_else(&incompatible_target)?;
				let values = target.values().i32_values().ok_or_else(&incompatible_target)?; prepare_indexed_labels( dataset,
					target, values, dictionary.iter().cloned().map(KnnLabelValue::bytes).collect(), ) }
			(SemanticType::Ordinal, VectorEncoding::OrdinalI32) => {
				let ordered_labels = target.metadata().ordinal_labels().ok_or_else(&incompatible_target)?;
				let values = target.values().i32_values().ok_or_else(&incompatible_target)?; prepare_indexed_labels( dataset,
					target, values, ordered_labels.iter().cloned().map(KnnLabelValue::bytes).collect(), ) }
			(SemanticType::Temporal, VectorEncoding::RelativeSecondsI32) if target.metadata().temporal_origin().is_some() =>
			{ let values = target.values().i32_values().ok_or_else(&incompatible_target)?; prepare_remapped_labels( dataset,
					target, |position, source_row| { return values.get(position).copied().ok_or_else(|| {
							return knn_target_error(target, source_row, "value is absent from prepared storage");
						}); }, KnnLabelValue::i32, ) }
			(SemanticType::Text, VectorEncoding::Utf8) if target.metadata().is_none() => prepare_variable_output(),
			(SemanticType::Binary, VectorEncoding::Bytes) if target.metadata().is_none() => prepare_variable_output(),
			(SemanticType::Image, VectorEncoding::Bytes) if target.metadata().is_image() => prepare_variable_output(),
			_ => Err(incompatible_target()), }; outputs.push(output?); }

	return Ok(KnnReferenceSet { neighbors, vectors: dataset .vectors() .iter() .map(|vector| {
				return CheckpointArtifactVector::from_schema(&vector.schema()); }) .collect(),
		feature_spans: feature_plan.spans().to_vec(),
		normalization_mask: feature_plan.normalization_mask().map(<[u32]>::to_vec),
		reference_source_rows: dataset.train().source_rows().to_vec(), reference_rows, feature_width, reference_feature_bits,
		outputs, }); }

/// Prepare one numeric target using a semantic-value to exact `f32`-bits conversion.
#[inline]
fn prepare_numeric_output<Value: Copy>( dataset: &PreparedDataset, target: &PreparedVector, values: &[Option<Value>],
	mut encode: impl FnMut(Value, usize) -> TrainingCompileResult<u32>, ) -> TrainingCompileResult<KnnReferenceOutput> {
	let mut target_bits = Vec::with_capacity(dataset.train().len());
	let mut known = Vec::with_capacity(dataset.train().len()); for (position, source_row) in partition_rows(dataset) {
		if let Some(value) = values .get(position) .copied() .ok_or_else(|| {
				return knn_target_error(target, source_row, "value is absent from prepared storage");
			})?
		{ target_bits.push(encode(value, source_row)?); known.push(1); } else { target_bits.push(0.0f32.to_bits());
			known.push(0); } }
	return finish_output(target, known, KnnReferenceValues::from_numeric_f32_bits(target_bits)); }

/// Fit an ordered label dictionary and remap one target through checked class codes.
#[inline]
fn prepare_remapped_labels<Value: Clone + Ord>( dataset: &PreparedDataset, target: &PreparedVector,
	mut value_at: impl FnMut(usize, usize) -> TrainingCompileResult<Option<Value>>,
	mut label_value: impl FnMut(Value) -> KnnLabelValue, ) -> TrainingCompileResult<KnnReferenceOutput> {
	let mut unique = BTreeSet::new(); for (position, source_row) in partition_rows(dataset) {
		if let Some(value) = value_at(position, source_row)? { unique.insert(value); } }
	let ordered_values = unique.into_iter().collect::<Vec<_>>(); let labels = ordered_values .iter() .cloned()
		.map(&mut label_value) .collect::<Vec<_>>(); let codes_by_value = ordered_values .into_iter() .enumerate()
		.map(|(index, value)| { let code = i32::try_from(index).map_err(|error| { return knn_target_error( target, 0,
					format!("label index {index} does not fit int32: {error}"),
				); })?; return Ok((value, code)); }) .collect::<TrainingCompileResult<BTreeMap<_, _>>>()?;
	let mut codes = Vec::with_capacity(dataset.train().len()); let mut known = Vec::with_capacity(dataset.train().len());
	for (position, source_row) in partition_rows(dataset) { if let Some(value) = value_at(position, source_row)? {
			let code = codes_by_value.get(&value).copied().ok_or_else(|| {
				return knn_target_error(target, source_row, "known label has no fitted class code");
			})?; codes.push(code); known.push(1); } else { codes.push(0); known.push(0); } }
	return finish_output( target, known, KnnReferenceValues::from_discrete_i32(codes, labels), ); }

/// Validate and retain an already indexed categorical or ordinal target.
fn prepare_indexed_labels( dataset: &PreparedDataset, target: &PreparedVector, values: &[Option<i32>],
	labels: Vec<KnnLabelValue>, ) -> TrainingCompileResult<KnnReferenceOutput> {
	validate_label_dictionary(target, &labels)?; let mut codes = Vec::with_capacity(dataset.train().len());
	let mut known = Vec::with_capacity(dataset.train().len()); for (position, source_row) in partition_rows(dataset) {
		if let Some(code) = values .get(position) .copied() .ok_or_else(|| {
				return knn_target_error(target, source_row, "value is absent from prepared storage");
			})?
		{ let index = usize::try_from(code).map_err(|conversion_error| { return knn_target_error( target, source_row,
					format!("negative label code {code}: {conversion_error}"),
				); })?; if index >= labels.len() { return Err(knn_target_error( target, source_row, format!(
						"label code {code} is outside {} fitted labels",
						labels.len() ), )); }
			codes.push(code); known.push(1); } else { codes.push(0); known.push(0); } }
	return finish_output( target, known, KnnReferenceValues::from_discrete_i32(codes, labels), ); }

/// Validate the calculation-facing bounds and uniqueness of a label dictionary.
fn validate_label_dictionary(target: &PreparedVector, labels: &[KnnLabelValue]) -> TrainingCompileResult<()> {
	if labels.is_empty() { return Err(knn_target_error( target, 0,
			"fitted label dictionary is empty",
		)); }
	if labels.len() > i32::MAX as usize { return Err(knn_target_error( target, 0,
			"fitted label dictionary exceeds the int32 calculation-code domain",
		)); }
	let mut unique = BTreeSet::new(); if labels.iter().any(|label| { return !unique.insert(label); }) {
		return Err(knn_target_error( target, 0,
			"fitted label dictionary contains duplicates",
		)); }
	return Ok(()); }

/// Finish one output after validating its known-row count and label dictionary.
fn finish_output( target: &PreparedVector, known: Vec<i32>, values: KnnReferenceValues,
) -> TrainingCompileResult<KnnReferenceOutput> { let known_references = u64::try_from(known.iter().filter(|value| {
		return **value == 1; }).count()).map_err(|error| { return knn_target_error( target, 0,
			format!("known-reference count does not fit u64: {error}"),
		); })?; if known_references == 0 { return Err(knn_target_error( target, 0,
			"training partition contains no known reference values",
		)); }
	if values.kind == KnnReferenceValueKind::DiscreteI32 { validate_label_dictionary(target, &values.labels)?; }
	return Ok(KnnReferenceOutput { schema: CheckpointArtifactVector::from_schema(&target.schema()), known,
		known_references, values, }); }

/// Convert an `i32` to `f32` only when its exact integer value is representable.
#[inline]
const fn exact_i32_as_f32(value: i32) -> Option<f32> { const SIGN_MASK: u32 = 1u32 << (u32::BITS - 1);
	const EXPONENT_BIAS: u32 = 127; const FRACTION_BITS: u32 = f32::MANTISSA_DIGITS - 1;

	if value == 0 { return Some(0.0); }
	let magnitude = value.unsigned_abs(); let exponent = (u32::BITS - 1) - magnitude.leading_zeros();
	let discarded_bits = exponent.saturating_sub(FRACTION_BITS);
	if discarded_bits != 0 && magnitude & ((1u32 << discarded_bits) - 1) != 0 { return None; }
	let significand = if exponent <= FRACTION_BITS { magnitude << (FRACTION_BITS - exponent) } else {
		magnitude >> discarded_bits }; let sign = if value.is_negative() { SIGN_MASK } else { 0 };
	let exponent_bits = (exponent + EXPONENT_BIAS) << FRACTION_BITS; let fraction_mask = (1u32 << FRACTION_BITS) - 1;
	return Some(f32::from_bits(sign | exponent_bits | (significand & fraction_mask))); }

/// Pair retained training positions with their original source-row indices.
fn partition_rows(dataset: &PreparedDataset) -> impl Iterator<Item = (usize, usize)> + '_ {
	return dataset .train() .retained_positions() .iter() .copied() .zip(dataset.train().source_rows().iter().copied()); }

/// Construct a target-specific KNN compilation error.
fn knn_target_error( target: &PreparedVector, source_row: usize, detail: impl fmt::Display, ) -> TrainingCompileError {
	return knn_error( TrainingCompileErrorKind::InvalidTargetMatrix, format!(
			"KNN target {:?} at source row {source_row}: {detail}",
			String::from_utf8_lossy(target.name()) ), ); }

/// Construct a KNN compilation error with the supplied classification.
fn knn_error(kind: TrainingCompileErrorKind, detail: impl Into<String>) -> TrainingCompileError {
	return TrainingCompileError::new(kind, detail); }
