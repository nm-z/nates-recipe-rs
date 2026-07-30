use core::num::NonZeroU64;
use std::error::Error;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use recipe_core::DType;
use recipe_ingest::{
	CategoricalEncodingModel, Delimiter, HeaderMode, IngestLimits, PreparationRequest, RawTable, TableRequest,
	TrainFraction, parse_table, prepare_table,
};
use recipe_training::{
	DenseActivation, DenseDataNormalization, DenseOperation, InferenceCompileErrorKind, InferenceInputRole,
	KnnInferencePredictionKind, KnnModelArtifact, KnnModelDecodeLimits, KnnReferenceSet,
	compile_prepared_knn_inference, load_knn_model_file, prepare_knn_inference_table, prepare_knn_reference_set,
};

type TestResult = Result<(), Box<dyn Error>>;

static NEXT_MODEL: AtomicU64 = AtomicU64::new(0);

struct ModelPath(PathBuf);

impl ModelPath {
	fn new(extension: &str) -> Self {
		let sequence = NEXT_MODEL.fetch_add(1, Ordering::Relaxed);
		Self(std::env::temp_dir().join(format!(
			"recipe-knn-model-{}-{sequence}.{extension}",
			std::process::id()
		)))
	}
}

impl Drop for ModelPath {
	fn drop(&mut self) {
		let _ = std::fs::remove_file(&self.0);
	}
}

fn table(source: &[u8]) -> Result<RawTable, Box<dyn Error>> {
	Ok(parse_table(
		source,
		TableRequest::new(
			Delimiter::Comma,
			HeaderMode::Present,
			IngestLimits::new(1_048_576, 64, 16, 4_096)?,
		),
	)?)
}

fn references() -> Result<KnnReferenceSet, Box<dyn Error>> {
	let source = table(b"amount,color,numeric_target,class_target\n\
		1,red,1.5,cat\n\
		2,blue,2.5,dog\n\
		3,red,3.5,cat\n\
		4,blue,4.5,dog\n\
		5,green,5.5,fox\n")?;
	let prepared = prepare_table(
		&source,
		&PreparationRequest::new(
			["class_target", "numeric_target"],
			TrainFraction::new(4, 5)?,
		),
		&CategoricalEncodingModel,
	)?;
	Ok(prepare_knn_reference_set(
		&prepared,
		NonZeroU64::new(3).unwrap(),
	)?)
}

fn queries() -> Result<RawTable, Box<dyn Error>> {
	table(b"color,unused,amount\n\
		blue,x,2\n\
		red,y,3\n\
		green,z,4\n")
}

#[test]
fn compiles_one_typed_prediction_for_every_declared_target() -> TestResult {
	let artifact = KnnModelArtifact::new(references()?, None, [])?;
	let prepared = prepare_knn_inference_table(artifact, &queries()?)?;
	let compiled = compile_prepared_knn_inference(&prepared)?;

	compiled.graph().validate()?;
	assert_eq!(compiled.rows(), 3);
	assert_eq!(compiled.outputs().len(), 2);
	assert_eq!(
		compiled
			.outputs()
			.iter()
			.map(|output| (output.dtype(), output.kind(), output.shape().extents()))
			.collect::<Vec<_>>(),
		[
			(
				DType::I32,
				KnnInferencePredictionKind::DiscreteMode,
				[3, 1].as_slice()
			),
			(
				DType::F32,
				KnnInferencePredictionKind::NumericMean,
				[3, 1].as_slice()
			),
		]
	);
	assert_eq!(
		compiled
			.graph()
			.tensors
			.iter()
			.filter(|tensor| tensor.external_output)
			.map(|tensor| tensor.dtype)
			.collect::<Vec<_>>(),
		[DType::I32, DType::F32]
	);
	assert_eq!(
		compiled
			.external_inputs()
			.iter()
			.filter(|input| matches!(input.role(), InferenceInputRole::KnnReferenceValues { .. }))
			.count(),
		2
	);
	assert_eq!(
		compiled
			.external_inputs()
			.iter()
			.filter(|input| matches!(input.role(), InferenceInputRole::KnnReferenceKnown { .. }))
			.count(),
		2
	);
	Ok(())
}

#[test]
fn compiles_every_declared_data_normalization_on_device() -> TestResult {
	let references = references()?;
	for normalization in [
		DenseDataNormalization::ZScore,
		DenseDataNormalization::MinMax,
		DenseDataNormalization::L2Norm,
	] {
		let artifact = KnnModelArtifact::new(references.clone(), Some(normalization), [])?;
		let prepared = prepare_knn_inference_table(artifact, &queries()?)?;
		let compiled = compile_prepared_knn_inference(&prepared)?;
		compiled.graph().validate()?;
		assert_eq!(compiled.outputs().len(), 2);
		assert!(
			compiled
				.external_inputs()
				.iter()
				.any(|input| { matches!(input.role(), InferenceInputRole::FeatureNormalizationMask) })
		);
	}
	Ok(())
}

#[test]
fn rejects_post_knn_operations_instead_of_transforming_class_codes() -> TestResult {
	let artifact = KnnModelArtifact::new(
		references()?,
		None,
		[DenseOperation::Activation(DenseActivation::Relu)],
	)?;
	let prepared = prepare_knn_inference_table(artifact, &queries()?)?;
	let error = compile_prepared_knn_inference(&prepared).unwrap_err();
	assert_eq!(error.kind(), InferenceCompileErrorKind::UnsupportedTopology);
	assert!(error.detail().contains("independently typed"));
	Ok(())
}

#[test]
fn saves_and_loads_one_private_canonical_semantic_model() -> TestResult {
	let artifact = KnnModelArtifact::new(references()?, Some(DenseDataNormalization::ZScore), [])?;
	let path = ModelPath::new("ogdl");
	artifact.save(&path.0)?;

	let metadata = std::fs::metadata(&path.0)?;
	assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
	let loaded = load_knn_model_file(&path.0, KnnModelDecodeLimits::default())?;
	assert_eq!(loaded, artifact);
	assert_eq!(std::fs::read(&path.0)?, loaded.encode()?);

	let invalid = ModelPath::new("bin");
	assert!(artifact.save(&invalid.0).is_err());
	assert!(!invalid.0.exists());
	Ok(())
}
