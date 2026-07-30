use core::num::NonZeroU64;
use std::collections::{BTreeMap, BTreeSet};

use recipe_ingest::{
	DenseMatrix, PartitionKind, PreparedDataset, PreparedValues, PreparedVector, SemanticType, VectorEncoding,
	VectorMetadata, VectorRole,
};
use recipe_ops::KnnOutputSpec;

use crate::model::{DenseFeaturePlan, lower_dense_features};
use crate::{
	CheckpointArtifactVector, CompiledFeatureSpan, TrainingCompileError, TrainingCompileErrorKind,
	TrainingCompileResult,
};

/// Exact semantic value represented by one calculation-facing KNN class code.
///
/// Numeric targets are reduced as f32 means and therefore do not use this
/// dictionary. Every nonnumeric target is reduced as a mode over deterministic
/// int32 codes, then decoded through one of these exact values.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum KnnLabelValue {
	I32(i32),
	Bytes(Vec<u8>),
}

/// Calculation-facing reference values for one declared target.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KnnReferenceValues {
	/// Exact finite f32 bit patterns. Missing rows contain positive zero and are
	/// excluded by the sibling known-value mask.
	NumericF32Bits(Vec<u32>),
	/// Canonical int32 codes into `labels`. Missing rows contain code zero and
	/// are excluded by the sibling known-value mask.
	DiscreteI32 {
		codes: Vec<i32>,
		labels: Vec<KnnLabelValue>,
	},
}

/// One independently reduced KNN output in target declaration order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnnReferenceOutput {
	pub(crate) schema: CheckpointArtifactVector,
	pub(crate) known: Vec<i32>,
	pub(crate) known_references: u64,
	pub(crate) values: KnnReferenceValues,
}

impl KnnReferenceOutput {
	#[must_use]
	pub const fn schema(&self) -> &CheckpointArtifactVector {
		&self.schema
	}

	#[must_use]
	pub fn known(&self) -> &[i32] {
		&self.known
	}

	#[must_use]
	pub const fn known_references(&self) -> u64 {
		self.known_references
	}

	#[must_use]
	pub const fn values(&self) -> &KnnReferenceValues {
		&self.values
	}

	#[must_use]
	pub fn operation_spec(&self) -> KnnOutputSpec {
		match &self.values {
			KnnReferenceValues::NumericF32Bits(_) => KnnOutputSpec::Numeric {
				known_references: self.known_references,
			},
			KnnReferenceValues::DiscreteI32 { labels, .. } => KnnOutputSpec::Categorical {
				known_references: self.known_references,
				classes: u64::try_from(labels.len()).expect("validated KNN label count fits u64"),
			},
		}
	}

	/// Decode one discrete prediction. Numeric outputs have no class decoder.
	#[must_use]
	pub fn decode_class(&self, code: i32) -> Option<&KnnLabelValue> {
		let KnnReferenceValues::DiscreteI32 { labels, .. } = &self.values else {
			return None;
		};
		usize::try_from(code)
			.ok()
			.and_then(|index| labels.get(index))
	}
}

/// Loss-independent, immutable KNN state prepared from the exact training
/// partition.
///
/// Features use the same typed scalar/one-hot lowering as dense execution but
/// remain unnormalized here. Numerical normalization and its fitted state
/// belong to the eventual execution/checkpoint boundary. Reference rows retain
/// prepared order, which is the deterministic distance-tie order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnnReferenceSet {
	pub(crate) neighbors: NonZeroU64,
	pub(crate) vectors: Vec<CheckpointArtifactVector>,
	pub(crate) feature_spans: Vec<CompiledFeatureSpan>,
	pub(crate) normalization_mask: Option<Vec<u32>>,
	pub(crate) reference_source_rows: Vec<usize>,
	pub(crate) reference_rows: usize,
	pub(crate) feature_width: usize,
	pub(crate) reference_feature_bits: Vec<u32>,
	pub(crate) outputs: Vec<KnnReferenceOutput>,
}

impl KnnReferenceSet {
	#[must_use]
	pub const fn neighbors(&self) -> NonZeroU64 {
		self.neighbors
	}

	#[must_use]
	pub fn vectors(&self) -> &[CheckpointArtifactVector] {
		&self.vectors
	}

	#[must_use]
	pub fn feature_spans(&self) -> &[CompiledFeatureSpan] {
		&self.feature_spans
	}

	#[must_use]
	pub fn normalization_mask(&self) -> Option<&[u32]> {
		self.normalization_mask.as_deref()
	}

	#[must_use]
	pub fn reference_source_rows(&self) -> &[usize] {
		&self.reference_source_rows
	}

	#[must_use]
	pub const fn reference_rows(&self) -> usize {
		self.reference_rows
	}

	#[must_use]
	pub const fn feature_width(&self) -> usize {
		self.feature_width
	}

	#[must_use]
	pub fn reference_feature_bits(&self) -> &[u32] {
		&self.reference_feature_bits
	}

	#[must_use]
	pub fn outputs(&self) -> &[KnnReferenceOutput] {
		&self.outputs
	}

	#[must_use]
	pub fn operation_specs(&self) -> Vec<KnnOutputSpec> {
		self.outputs
			.iter()
			.map(KnnReferenceOutput::operation_spec)
			.collect()
	}
}

/// Prepare the immutable reference state consumed by one public
/// `.knn(neighbors)` declaration.
///
/// Exactly one output is emitted for every declared target, in declaration
/// order. Numeric outputs use a uniform mean. Every nonnumeric semantic type
/// uses a uniform mode over an exact deterministic label dictionary. Missing
/// references are retained in row alignment but excluded independently for
/// each output.
pub fn prepare_knn_reference_set(
	dataset: &PreparedDataset,
	neighbors: NonZeroU64,
) -> TrainingCompileResult<KnnReferenceSet> {
	if dataset.train().is_empty() {
		return Err(knn_error(
			TrainingCompileErrorKind::EmptyDataset,
			"KNN requires at least one prepared training reference row",
		));
	}
	if dataset.target_source_indices().is_empty() {
		return Err(knn_error(
			TrainingCompileErrorKind::InvalidTargetMatrix,
			"KNN requires at least one declared target",
		));
	}

	let feature_plan = DenseFeaturePlan::from_prepared(dataset)?;
	let lowered = lower_dense_features(dataset, &feature_plan, PartitionKind::Train)?;
	let (reference_rows, feature_width, reference_feature_bits) = f32_reference_features(lowered)?;
	let targets = dataset
		.vectors()
		.iter()
		.filter(|vector| vector.role() == VectorRole::Target)
		.map(|vector| (vector.source_index(), vector))
		.collect::<BTreeMap<_, _>>();
	let mut seen_targets = BTreeSet::new();
	let mut outputs = Vec::with_capacity(dataset.target_source_indices().len());
	for source_index in dataset.target_source_indices().iter().copied() {
		if !seen_targets.insert(source_index) {
			return Err(knn_error(
				TrainingCompileErrorKind::InvalidTargetMatrix,
				format!("KNN target source index {source_index} is declared more than once"),
			));
		}
		let target = targets.get(&source_index).copied().ok_or_else(|| {
			knn_error(
				TrainingCompileErrorKind::InvalidTargetMatrix,
				format!("KNN declared target source index {source_index} is absent from prepared vectors"),
			)
		})?;
		outputs.push(prepare_output(dataset, target)?);
	}

	Ok(KnnReferenceSet {
		neighbors,
		vectors: dataset
			.vectors()
			.iter()
			.map(|vector| CheckpointArtifactVector::from_schema(&vector.schema()))
			.collect(),
		feature_spans: feature_plan.spans().to_vec(),
		normalization_mask: feature_plan.normalization_mask().map(<[u32]>::to_vec),
		reference_source_rows: dataset.train().source_rows().to_vec(),
		reference_rows,
		feature_width,
		reference_feature_bits,
		outputs,
	})
}

fn f32_reference_features(matrix: DenseMatrix) -> TrainingCompileResult<(usize, usize, Vec<u32>)> {
	match matrix {
		DenseMatrix::I32 {
			rows,
			columns,
			values,
		} => {
			let bits = values
				.into_iter()
				.enumerate()
				.map(|(index, value)| {
					let converted = value as f32;
					if f64::from(converted) != f64::from(value) {
						return Err(knn_error(
							TrainingCompileErrorKind::InvalidFeatureMatrix,
							format!(
								"KNN feature element {index} int32 value {value} is not exactly representable as f32"
							),
						));
					}
					Ok(converted.to_bits())
				})
				.collect::<TrainingCompileResult<Vec<_>>>()?;
			Ok((rows, columns, bits))
		}
		DenseMatrix::F32Bits {
			rows,
			columns,
			values,
		} => {
			for (index, bits) in values.iter().copied().enumerate() {
				if !f32::from_bits(bits).is_finite() {
					return Err(knn_error(
						TrainingCompileErrorKind::InvalidFeatureMatrix,
						format!("KNN feature element {index} has non-finite f32 bits {bits:#010x}"),
					));
				}
			}
			Ok((rows, columns, values))
		}
	}
}

fn prepare_output(dataset: &PreparedDataset, target: &PreparedVector) -> TrainingCompileResult<KnnReferenceOutput> {
	match (
		target.semantic_type(),
		target.encoding(),
		target.metadata(),
		target.values(),
	) {
		(SemanticType::Numeric, VectorEncoding::I32, VectorMetadata::None, PreparedValues::I32(values)) => {
			prepare_numeric_i32(dataset, target, values)
		}
		(SemanticType::Numeric, VectorEncoding::F32, VectorMetadata::None, PreparedValues::F32Bits(values)) => {
			prepare_numeric_f32(dataset, target, values)
		}
		(
			SemanticType::Categorical,
			VectorEncoding::DictionaryI32,
			VectorMetadata::Categorical { dictionary },
			PreparedValues::I32(values),
		) => prepare_indexed_labels(
			dataset,
			target,
			values,
			dictionary
				.iter()
				.cloned()
				.map(KnnLabelValue::Bytes)
				.collect(),
		),
		(
			SemanticType::Ordinal,
			VectorEncoding::OrdinalI32,
			VectorMetadata::Ordinal { ordered_labels },
			PreparedValues::I32(values),
		) => prepare_indexed_labels(
			dataset,
			target,
			values,
			ordered_labels
				.iter()
				.cloned()
				.map(KnnLabelValue::Bytes)
				.collect(),
		),
		(
			SemanticType::Temporal,
			VectorEncoding::RelativeSecondsI32,
			VectorMetadata::Temporal { .. },
			PreparedValues::I32(values),
		) => prepare_temporal_labels(dataset, target, values),
		(SemanticType::Text, VectorEncoding::Utf8, VectorMetadata::None, PreparedValues::VariableWidth(values))
		| (
			SemanticType::Binary,
			VectorEncoding::Bytes,
			VectorMetadata::None,
			PreparedValues::VariableWidth(values),
		)
		| (
			SemanticType::Image,
			VectorEncoding::Bytes,
			VectorMetadata::Image { .. },
			PreparedValues::VariableWidth(values),
		) => prepare_byte_labels(dataset, target, values),
		_ => Err(knn_target_error(
			target,
			0,
			format!(
				"incompatible prepared semantic tuple {:?}/{:?}/{:?}",
				target.semantic_type(),
				target.encoding(),
				target.metadata()
			),
		)),
	}
}

fn prepare_numeric_i32(
	dataset: &PreparedDataset,
	target: &PreparedVector,
	values: &[Option<i32>],
) -> TrainingCompileResult<KnnReferenceOutput> {
	let mut lowered = Vec::with_capacity(dataset.train().len());
	let mut known = Vec::with_capacity(dataset.train().len());
	for (position, source_row) in partition_rows(dataset) {
		match values
			.get(position)
			.ok_or_else(|| knn_target_error(target, source_row, "value is absent from prepared storage"))?
		{
			Some(value) => {
				let converted = *value as f32;
				if f64::from(converted) != f64::from(*value) {
					return Err(knn_target_error(
						target,
						source_row,
						format!("int32 value {value} is not exactly representable as f32"),
					));
				}
				lowered.push(converted.to_bits());
				known.push(1);
			}
			None => {
				lowered.push(0.0_f32.to_bits());
				known.push(0);
			}
		}
	}
	finish_output(target, known, KnnReferenceValues::NumericF32Bits(lowered))
}

fn prepare_numeric_f32(
	dataset: &PreparedDataset,
	target: &PreparedVector,
	values: &[Option<u32>],
) -> TrainingCompileResult<KnnReferenceOutput> {
	let mut lowered = Vec::with_capacity(dataset.train().len());
	let mut known = Vec::with_capacity(dataset.train().len());
	for (position, source_row) in partition_rows(dataset) {
		match values
			.get(position)
			.ok_or_else(|| knn_target_error(target, source_row, "value is absent from prepared storage"))?
		{
			Some(bits) if f32::from_bits(*bits).is_finite() => {
				lowered.push(*bits);
				known.push(1);
			}
			Some(bits) => {
				return Err(knn_target_error(
					target,
					source_row,
					format!("value has non-finite f32 bits {bits:#010x}"),
				));
			}
			None => {
				lowered.push(0.0_f32.to_bits());
				known.push(0);
			}
		}
	}
	finish_output(target, known, KnnReferenceValues::NumericF32Bits(lowered))
}

fn prepare_indexed_labels(
	dataset: &PreparedDataset,
	target: &PreparedVector,
	values: &[Option<i32>],
	labels: Vec<KnnLabelValue>,
) -> TrainingCompileResult<KnnReferenceOutput> {
	validate_label_dictionary(target, &labels)?;
	let mut codes = Vec::with_capacity(dataset.train().len());
	let mut known = Vec::with_capacity(dataset.train().len());
	for (position, source_row) in partition_rows(dataset) {
		match values
			.get(position)
			.ok_or_else(|| knn_target_error(target, source_row, "value is absent from prepared storage"))?
		{
			Some(code) => {
				let index = usize::try_from(*code).map_err(|_| {
					knn_target_error(target, source_row, format!("negative label code {code}"))
				})?;
				if index >= labels.len() {
					return Err(knn_target_error(
						target,
						source_row,
						format!(
							"label code {code} is outside {} fitted labels",
							labels.len()
						),
					));
				}
				codes.push(*code);
				known.push(1);
			}
			None => {
				codes.push(0);
				known.push(0);
			}
		}
	}
	finish_output(
		target,
		known,
		KnnReferenceValues::DiscreteI32 { codes, labels },
	)
}

fn prepare_temporal_labels(
	dataset: &PreparedDataset,
	target: &PreparedVector,
	values: &[Option<i32>],
) -> TrainingCompileResult<KnnReferenceOutput> {
	let mut unique = BTreeSet::new();
	for (position, source_row) in partition_rows(dataset) {
		if let Some(value) = values
			.get(position)
			.ok_or_else(|| knn_target_error(target, source_row, "value is absent from prepared storage"))?
		{
			unique.insert(*value);
		}
	}
	let labels = unique
		.into_iter()
		.map(KnnLabelValue::I32)
		.collect::<Vec<_>>();
	let codes = labels
		.iter()
		.enumerate()
		.map(|(index, label)| Ok((label.clone(), checked_class_code(target, index)?)))
		.collect::<TrainingCompileResult<BTreeMap<_, _>>>()?;
	prepare_remapped_i32(dataset, target, values, labels, &codes)
}

fn prepare_remapped_i32(
	dataset: &PreparedDataset,
	target: &PreparedVector,
	values: &[Option<i32>],
	labels: Vec<KnnLabelValue>,
	codes_by_label: &BTreeMap<KnnLabelValue, i32>,
) -> TrainingCompileResult<KnnReferenceOutput> {
	let mut codes = Vec::with_capacity(dataset.train().len());
	let mut known = Vec::with_capacity(dataset.train().len());
	for (position, source_row) in partition_rows(dataset) {
		match values
			.get(position)
			.ok_or_else(|| knn_target_error(target, source_row, "value is absent from prepared storage"))?
		{
			Some(value) => {
				let code = codes_by_label
					.get(&KnnLabelValue::I32(*value))
					.copied()
					.ok_or_else(|| {
						knn_target_error(target, source_row, "known temporal label has no class code")
					})?;
				codes.push(code);
				known.push(1);
			}
			None => {
				codes.push(0);
				known.push(0);
			}
		}
	}
	finish_output(
		target,
		known,
		KnnReferenceValues::DiscreteI32 { codes, labels },
	)
}

fn prepare_byte_labels(
	dataset: &PreparedDataset,
	target: &PreparedVector,
	values: &recipe_ingest::VariableWidthVector,
) -> TrainingCompileResult<KnnReferenceOutput> {
	let mut unique = BTreeSet::new();
	for (position, source_row) in partition_rows(dataset) {
		if let Some(value) = values.value(position).ok_or_else(|| {
			knn_target_error(
				target,
				source_row,
				"value is absent from variable-width storage",
			)
		})? {
			unique.insert(value.to_vec());
		}
	}
	let labels = unique
		.into_iter()
		.map(KnnLabelValue::Bytes)
		.collect::<Vec<_>>();
	let codes_by_label = labels
		.iter()
		.enumerate()
		.map(|(index, label)| Ok((label.clone(), checked_class_code(target, index)?)))
		.collect::<TrainingCompileResult<BTreeMap<_, _>>>()?;
	let mut codes = Vec::with_capacity(dataset.train().len());
	let mut known = Vec::with_capacity(dataset.train().len());
	for (position, source_row) in partition_rows(dataset) {
		match values.value(position).ok_or_else(|| {
			knn_target_error(
				target,
				source_row,
				"value is absent from variable-width storage",
			)
		})? {
			Some(value) => {
				let code = codes_by_label
					.get(&KnnLabelValue::Bytes(value.to_vec()))
					.copied()
					.ok_or_else(|| {
						knn_target_error(target, source_row, "known byte label has no class code")
					})?;
				codes.push(code);
				known.push(1);
			}
			None => {
				codes.push(0);
				known.push(0);
			}
		}
	}
	finish_output(
		target,
		known,
		KnnReferenceValues::DiscreteI32 { codes, labels },
	)
}

fn validate_label_dictionary(target: &PreparedVector, labels: &[KnnLabelValue]) -> TrainingCompileResult<()> {
	if labels.is_empty() {
		return Err(knn_target_error(
			target,
			0,
			"fitted label dictionary is empty",
		));
	}
	if labels.len() > i32::MAX as usize {
		return Err(knn_target_error(
			target,
			0,
			"fitted label dictionary exceeds the int32 calculation-code domain",
		));
	}
	let mut unique = BTreeSet::new();
	if labels.iter().any(|label| !unique.insert(label)) {
		return Err(knn_target_error(
			target,
			0,
			"fitted label dictionary contains duplicates",
		));
	}
	Ok(())
}

fn checked_class_code(target: &PreparedVector, index: usize) -> TrainingCompileResult<i32> {
	i32::try_from(index).map_err(|error| {
		knn_target_error(
			target,
			0,
			format!("label index {index} does not fit int32: {error}"),
		)
	})
}

fn finish_output(
	target: &PreparedVector,
	known: Vec<i32>,
	values: KnnReferenceValues,
) -> TrainingCompileResult<KnnReferenceOutput> {
	let known_references = u64::try_from(known.iter().filter(|value| **value == 1).count()).map_err(|error| {
		knn_target_error(
			target,
			0,
			format!("known-reference count does not fit u64: {error}"),
		)
	})?;
	if known_references == 0 {
		return Err(knn_target_error(
			target,
			0,
			"training partition contains no known reference values",
		));
	}
	if let KnnReferenceValues::DiscreteI32 { labels, .. } = &values {
		validate_label_dictionary(target, labels)?;
	}
	Ok(KnnReferenceOutput {
		schema: CheckpointArtifactVector::from_schema(&target.schema()),
		known,
		known_references,
		values,
	})
}

fn partition_rows(dataset: &PreparedDataset) -> impl Iterator<Item = (usize, usize)> + '_ {
	dataset
		.train()
		.retained_positions()
		.iter()
		.copied()
		.zip(dataset.train().source_rows().iter().copied())
}

fn knn_target_error(
	target: &PreparedVector,
	source_row: usize,
	detail: impl core::fmt::Display,
) -> TrainingCompileError {
	knn_error(
		TrainingCompileErrorKind::InvalidTargetMatrix,
		format!(
			"KNN target {:?} at source row {source_row}: {detail}",
			String::from_utf8_lossy(target.name())
		),
	)
}

fn knn_error(kind: TrainingCompileErrorKind, detail: impl Into<String>) -> TrainingCompileError {
	TrainingCompileError::new(kind, detail)
}
