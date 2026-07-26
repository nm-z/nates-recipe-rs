use core::fmt;
use std::path::Path;

use recipe_ingest::{
	DistilledDataset, InferenceFeatureEncoding, InferenceFeatureSchema, InferencePrepareError,
	PreparedInferenceDataset, PreparedInferenceFeature, PreparedInferenceValues, RawTable, SemanticType, SourceError,
	SourceLimit, VectorEncoding, read_source_snapshot,
};

use crate::{
	CheckpointArtifact, CheckpointArtifactMetadata, CheckpointDecodeLimits, CheckpointError, CheckpointTensorImage,
	CompiledFeatureSpan, DenseDataNormalization, DenseFeatureLowering, decode_checkpoint,
};

/// Failure to load a checkpoint or prepare schema-bound inference features.
#[derive(Debug)]
#[non_exhaustive]
pub enum InferencePreparationError {
	CheckpointSource(SourceError),
	Checkpoint(CheckpointError),
	Data(InferencePrepareError),
	InconsistentCheckpoint {
		feature: usize,
		source_vector: usize,
		detail: String,
	},
	ArithmeticOverflow {
		detail: String,
	},
}

impl fmt::Display for InferencePreparationError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::CheckpointSource(error) => write!(formatter, "load checkpoint source: {error}"),
			Self::Checkpoint(error) => write!(formatter, "load checkpoint: {error}"),
			Self::Data(error) => write!(formatter, "prepare inference data: {error}"),
			Self::InconsistentCheckpoint {
				feature,
				source_vector,
				detail,
			} => write!(
				formatter,
				"inconsistent checkpoint inference.feature[{feature}].source-vector[{source_vector}]: {detail}"
			),
			Self::ArithmeticOverflow { detail } => write!(formatter, "prepare inference data: {detail}"),
		}
	}
}

impl std::error::Error for InferencePreparationError {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match self {
			Self::CheckpointSource(error) => Some(error),
			Self::Checkpoint(error) => Some(error),
			Self::Data(error) => Some(error),
			Self::InconsistentCheckpoint { .. } | Self::ArithmeticOverflow { .. } => None,
		}
	}
}

impl From<SourceError> for InferencePreparationError {
	fn from(error: SourceError) -> Self {
		Self::CheckpointSource(error)
	}
}

impl From<CheckpointError> for InferencePreparationError {
	fn from(error: CheckpointError) -> Self {
		Self::Checkpoint(error)
	}
}

impl From<InferencePrepareError> for InferencePreparationError {
	fn from(error: InferencePrepareError) -> Self {
		Self::Data(error)
	}
}

pub type InferencePreparationResult<T> = Result<T, InferencePreparationError>;

/// A decoded model and its unnormalized, schema-bound inference rows.
///
/// The exact fitted normalization tensors remain in `checkpoint`; no host-side
/// numeric conversion, one-hot expansion, normalization, or model calculation
/// has been performed.
#[derive(Clone, Debug, PartialEq)]
pub struct PreparedInference {
	checkpoint: CheckpointArtifact,
	data: PreparedInferenceDataset,
}

impl PreparedInference {
	#[must_use]
	pub const fn checkpoint(&self) -> &CheckpointArtifact {
		&self.checkpoint
	}

	#[must_use]
	pub const fn data(&self) -> &PreparedInferenceDataset {
		&self.data
	}

	#[must_use]
	pub fn features(&self) -> &[PreparedInferenceFeature] {
		self.data.features()
	}

	#[must_use]
	pub fn feature_spans(&self) -> &[CompiledFeatureSpan] {
		self.checkpoint.feature_spans()
	}

	#[must_use]
	pub const fn normalization(&self) -> DenseDataNormalization {
		self.checkpoint.config().data_normalization
	}

	#[must_use]
	pub fn normalization_tensors(&self) -> &[CheckpointTensorImage] {
		self.checkpoint.normalization()
	}

	#[must_use]
	pub fn feature_normalization_mask(&self) -> &[u32] {
		self.checkpoint.feature_normalization_mask()
	}

	#[must_use]
	pub fn normalization_epsilon(&self) -> f32 {
		self.checkpoint.config().normalization_epsilon
	}

	#[must_use]
	pub fn into_checkpoint(self) -> CheckpointArtifact {
		self.checkpoint
	}
}

/// Read and decode one checkpoint from a bounded regular-file snapshot.
///
/// The file is opened and read only during this preparation call. Its admitted
/// bytes are then decoded by the strict checkpoint-v5 decoder.
///
/// # Errors
///
/// Returns a typed source error for a zero or unrepresentable bound, an I/O
/// failure, a non-regular source, or a file exceeding the bound. Strict v5
/// decoding errors retain their checkpoint paths.
pub fn load_checkpoint_file(
	path: impl AsRef<Path>,
	limits: CheckpointDecodeLimits,
) -> InferencePreparationResult<CheckpointArtifact> {
	let source_bytes =
		u64::try_from(limits.source_bytes).map_err(|error| InferencePreparationError::ArithmeticOverflow {
			detail: format!("checkpoint source-byte bound cannot be represented as u64: {error}"),
		})?;
	let source = read_source_snapshot(path.as_ref(), SourceLimit::new(source_bytes)?)?;
	decode_checkpoint(source.bytes(), limits).map_err(Into::into)
}

/// Prepare a newly distilled dataset under one decoded checkpoint's schema.
///
/// Only saved feature names are read. Source columns may be reordered, target
/// columns may be absent, and unrelated columns are ignored. Saved categorical
/// dictionaries and their v5 reserved route are reused exactly.
///
/// # Errors
///
/// Returns a typed schema/data error or a checked dense-lowering failure.
pub fn prepare_checkpoint_inference(
	checkpoint: CheckpointArtifact,
	dataset: &DistilledDataset,
) -> InferencePreparationResult<PreparedInference> {
	prepare_checkpoint_inference_table(checkpoint, dataset.table())
}

/// Prepare a framed table under one decoded checkpoint's schema.
///
/// This lower-level boundary is useful when a caller already owns an admitted
/// [`RawTable`]. It has the same semantics as [`prepare_checkpoint_inference`].
///
/// # Errors
///
/// Returns a typed schema/data error or a checked dense-lowering failure.
pub fn prepare_checkpoint_inference_table(
	checkpoint: CheckpointArtifact,
	table: &RawTable,
) -> InferencePreparationResult<PreparedInference> {
	let schema = saved_feature_schema(&checkpoint)?;
	let data = recipe_ingest::prepare_inference_table(table, &schema)?;
	validate_prepared_feature_spans(&data, checkpoint.feature_spans())?;
	Ok(PreparedInference { checkpoint, data })
}

/// Load a bounded checkpoint and prepare a newly distilled dataset in one call.
///
/// # Errors
///
/// Returns any checkpoint source, strict decoding, schema application, or dense
/// lowering failure.
pub fn load_and_prepare_checkpoint_inference(
	path: impl AsRef<Path>,
	limits: CheckpointDecodeLimits,
	dataset: &DistilledDataset,
) -> InferencePreparationResult<PreparedInference> {
	let checkpoint = load_checkpoint_file(path, limits)?;
	prepare_checkpoint_inference(checkpoint, dataset)
}

fn saved_feature_schema(checkpoint: &CheckpointArtifact) -> InferencePreparationResult<Vec<InferenceFeatureSchema>> {
	checkpoint
		.feature_spans()
		.iter()
		.enumerate()
		.map(|(feature, span)| {
			let vector = checkpoint
				.vectors()
				.iter()
				.find(|vector| vector.source_index() == span.source_vector())
				.ok_or_else(|| inconsistent_feature(feature, span, "saved feature vector is absent"))?;
			let encoding = match (
				span.lowering(),
				vector.semantic_type(),
				vector.encoding(),
				vector.metadata(),
			) {
				(
					DenseFeatureLowering::NumericScalar,
					SemanticType::Numeric,
					VectorEncoding::I32,
					CheckpointArtifactMetadata::None,
				) => InferenceFeatureEncoding::NumericI32,
				(
					DenseFeatureLowering::NumericScalar,
					SemanticType::Numeric,
					VectorEncoding::F32,
					CheckpointArtifactMetadata::None,
				) => InferenceFeatureEncoding::NumericF32,
				(
					DenseFeatureLowering::CategoricalOneHot {
						dictionary_width,
						reserved_index,
					},
					SemanticType::Categorical,
					VectorEncoding::DictionaryI32,
					CheckpointArtifactMetadata::Categorical { dictionary },
				) if dictionary_width == dictionary.len()
					&& reserved_index == dictionary.len()
					&& span.width() == dictionary.len().saturating_add(1) =>
				{
					InferenceFeatureEncoding::CategoricalDictionary {
						dictionary: dictionary.clone(),
					}
				}
				_ => {
					return Err(inconsistent_feature(
						feature,
						span,
						"saved vector schema and dense lowering are inconsistent",
					));
				}
			};
			Ok(InferenceFeatureSchema::new(
				span.source_vector(),
				vector.name(),
				encoding,
			))
		})
		.collect()
}

fn validate_prepared_feature_spans(
	prepared: &PreparedInferenceDataset,
	spans: &[CompiledFeatureSpan],
) -> InferencePreparationResult<()> {
	if prepared.features().len() != spans.len() {
		return Err(InferencePreparationError::InconsistentCheckpoint {
			feature: prepared.features().len().min(spans.len()),
			source_vector: spans
				.get(prepared.features().len().min(spans.len()))
				.map_or(0, CompiledFeatureSpan::source_vector),
			detail: "prepared feature count differs from the saved span count".to_owned(),
		});
	}
	for (feature, (prepared, span)) in prepared.features().iter().zip(spans).enumerate() {
		if prepared.schema().source_vector() != span.source_vector() {
			return Err(inconsistent_feature(
				feature,
				span,
				"prepared feature identity differs from the saved span",
			));
		}
		match (
			span.lowering(),
			prepared.schema().encoding(),
			prepared.values(),
		) {
			(
				DenseFeatureLowering::NumericScalar,
				InferenceFeatureEncoding::NumericI32,
				PreparedInferenceValues::I32(_),
			)
			| (
				DenseFeatureLowering::NumericScalar,
				InferenceFeatureEncoding::NumericF32,
				PreparedInferenceValues::F32Bits(_),
			) if span.width() == 1 => {}
			(
				DenseFeatureLowering::CategoricalOneHot {
					dictionary_width,
					reserved_index,
				},
				InferenceFeatureEncoding::CategoricalDictionary { dictionary },
				PreparedInferenceValues::I32(_),
			) if dictionary_width == dictionary.len()
				&& reserved_index == dictionary.len()
				&& span.width() == dictionary.len().saturating_add(1) => {}
			_ => {
				return Err(inconsistent_feature(
					feature,
					span,
					"prepared saved-schema values and feature span are inconsistent",
				));
			}
		}
	}
	Ok(())
}

fn inconsistent_feature(
	feature: usize,
	span: &CompiledFeatureSpan,
	detail: impl Into<String>,
) -> InferencePreparationError {
	InferencePreparationError::InconsistentCheckpoint {
		feature,
		source_vector: span.source_vector(),
		detail: detail.into(),
	}
}

#[cfg(test)]
mod tests {
	use std::fs;
	use std::sync::atomic::{AtomicU64, Ordering};

	use recipe_ingest::{
		Delimiter, HeaderMode, InferencePrepareErrorKind, IngestLimits, TableRequest, distill_dataset, parse_table,
	};

	use super::*;

	#[derive(Debug)]
	struct TestPath(std::path::PathBuf);

	impl TestPath {
		fn new(name: &str) -> Self {
			static NEXT: AtomicU64 = AtomicU64::new(1);
			let path = std::env::temp_dir().join(format!(
				"recipe-training-inference-{}-{}-{name}",
				std::process::id(),
				NEXT.fetch_add(1, Ordering::Relaxed)
			));
			Self(path)
		}
	}

	impl Drop for TestPath {
		fn drop(&mut self) {
			let _ = fs::remove_file(&self.0);
			let _ = fs::remove_dir_all(&self.0);
		}
	}

	fn table(source: &[u8]) -> RawTable {
		parse_table(
			source,
			TableRequest::new(
				Delimiter::Comma,
				HeaderMode::Present,
				IngestLimits::new(4_096, 32, 16, 1_024).unwrap(),
			),
		)
		.unwrap()
	}

	#[test]
	fn saved_spans_align_raw_numeric_and_reserved_categorical_routes_without_host_calculation() {
		let schema = [
			InferenceFeatureSchema::new(3, b"amount", InferenceFeatureEncoding::NumericI32),
			InferenceFeatureSchema::new(
				8,
				b"color",
				InferenceFeatureEncoding::CategoricalDictionary {
					dictionary: vec![b"blue".to_vec(), b"red".to_vec()],
				},
			),
		];
		let prepared =
			recipe_ingest::prepare_inference_table(&table(b"color,amount\nred,10\npurple,20\n"), &schema)
				.unwrap();
		let spans = [
			CompiledFeatureSpan::new(3, 0, 1, DenseFeatureLowering::NumericScalar),
			CompiledFeatureSpan::new(
				8,
				1,
				3,
				DenseFeatureLowering::CategoricalOneHot {
					dictionary_width: 2,
					reserved_index: 2,
				},
			),
		];
		validate_prepared_feature_spans(&prepared, &spans).unwrap();
		assert_eq!(
			prepared.features()[0].values(),
			&PreparedInferenceValues::I32(vec![10, 20])
		);
		assert_eq!(
			prepared.features()[1].values(),
			&PreparedInferenceValues::I32(vec![1, 2])
		);
	}

	#[test]
	fn checkpoint_file_loader_enforces_regular_file_and_source_bound() {
		let directory = TestPath::new("directory");
		fs::create_dir(&directory.0).unwrap();
		let error = load_checkpoint_file(&directory.0, CheckpointDecodeLimits::default()).unwrap_err();
		assert!(matches!(
			error,
			InferencePreparationError::CheckpointSource(_)
		));

		let file = TestPath::new("large.ogdl");
		fs::write(&file.0, b"12345").unwrap();
		let mut limits = CheckpointDecodeLimits::default();
		limits.source_bytes = 4;
		let error = load_checkpoint_file(&file.0, limits).unwrap_err();
		assert!(matches!(
			error,
			InferencePreparationError::CheckpointSource(_)
		));
	}

	#[test]
	fn loaded_v5_artifact_prepares_reordered_target_free_rows_and_retains_normalization() {
		let file = TestPath::new("valid.ogdl");
		fs::write(
			&file.0,
			crate::checkpoint::encoded_test_checkpoint_fixture(),
		)
		.unwrap();
		let checkpoint = load_checkpoint_file(&file.0, CheckpointDecodeLimits::default()).unwrap();
		let data_file = TestPath::new("inference.csv");
		fs::write(&data_file.0, b"extra,\"feature\nbytes\"\nignored,1.5\n").unwrap();
		let dataset = distill_dataset(
			&data_file.0,
			IngestLimits::new(4_096, 32, 16, 1_024).unwrap(),
		)
		.unwrap();
		let prepared = prepare_checkpoint_inference(checkpoint, &dataset).unwrap();
		assert_eq!(
			prepared.features()[0].values(),
			&PreparedInferenceValues::F32Bits(vec![1.5f32.to_bits()])
		);
		assert_eq!(prepared.feature_spans().len(), 1);
		assert_eq!(prepared.normalization(), DenseDataNormalization::ZScore);
		assert_eq!(prepared.normalization_tensors().len(), 2);
		assert_eq!(prepared.feature_normalization_mask(), &[1.0f32.to_bits()]);
		assert_eq!(prepared.normalization_epsilon().to_bits(), 0x3586_37bd);
	}

	#[test]
	fn loaded_v5_dictionary_routes_unseen_and_missing_values_without_refitting() {
		let file = TestPath::new("categorical.ogdl");
		fs::write(
			&file.0,
			crate::checkpoint::encoded_categorical_feature_test_checkpoint_fixture(),
		)
		.unwrap();
		let checkpoint = load_checkpoint_file(&file.0, CheckpointDecodeLimits::default()).unwrap();
		let data_file = TestPath::new("categorical-inference.csv");
		fs::write(
			&data_file.0,
			b"extra,\"feature\nbytes\"\nignored,red\nignored,purple\nignored,\"\"\n",
		)
		.unwrap();
		let dataset = distill_dataset(
			&data_file.0,
			IngestLimits::new(4_096, 32, 16, 1_024).unwrap(),
		)
		.unwrap();
		let prepared = prepare_checkpoint_inference(checkpoint, &dataset).unwrap();
		assert_eq!(
			prepared.features()[0].values(),
			&PreparedInferenceValues::I32(vec![1, 2, 2])
		);
		assert_eq!(prepared.feature_spans()[0].start(), 0);
		assert_eq!(prepared.feature_spans()[0].width(), 3);
		assert_eq!(
			prepared.feature_normalization_mask(),
			&[0.0f32.to_bits(), 0.0f32.to_bits(), 0.0f32.to_bits()]
		);
	}

	#[test]
	fn loaded_v5_schema_rejects_a_missing_required_feature_with_its_typed_path() {
		let file = TestPath::new("missing-feature.ogdl");
		fs::write(
			&file.0,
			crate::checkpoint::encoded_test_checkpoint_fixture(),
		)
		.unwrap();
		let checkpoint = load_checkpoint_file(&file.0, CheckpointDecodeLimits::default()).unwrap();
		let error = prepare_checkpoint_inference_table(checkpoint, &table(b"extra\n1\n")).unwrap_err();
		let InferencePreparationError::Data(error) = error else {
			panic!("expected typed inference data error");
		};
		assert_eq!(
			error.kind(),
			InferencePrepareErrorKind::MissingRequiredFeature
		);
		let path = error.path().unwrap();
		assert_eq!(path.feature(), 0);
		assert_eq!(path.source_vector(), 0);
		assert_eq!(path.column(), b"feature\nbytes");
		assert_eq!(path.source_row(), None);
	}

	#[test]
	fn checkpoint_schema_errors_remain_typed_through_training_boundary() {
		let schema = [InferenceFeatureSchema::new(
			4,
			b"required",
			InferenceFeatureEncoding::NumericF32,
		)];
		let error = recipe_ingest::prepare_inference_table(&table(b"other\n1\n"), &schema).unwrap_err();
		let error = InferencePreparationError::from(error);
		let InferencePreparationError::Data(error) = error else {
			panic!("expected typed inference data error");
		};
		assert_eq!(
			error.kind(),
			InferencePrepareErrorKind::MissingRequiredFeature
		);
		assert_eq!(error.path().unwrap().source_vector(), 4);
	}
}
