use core::num::NonZeroU64;
use std::error::Error;

use recipe_ingest::{
	CategoricalEncodingModel, Delimiter, HeaderMode, IngestLimits, PreparationRequest, PreparedDataset, TableRequest,
	TrainFraction, infer_table_vectors, parse_table, prepare_inferred_table, prepare_table,
};
use recipe_ops::KnnOutputSpec;
use recipe_training::{
	CheckpointDecodeErrorKind, CheckpointError, DenseActivation, DenseDataNormalization, DenseNormalization,
	DenseOperation, KnnLabelValue, KnnModelArtifact, KnnModelDecodeLimits, KnnReferenceValues,
	TrainingCompileErrorKind, prepare_knn_reference_set,
};

type TestResult = Result<(), Box<dyn Error>>;

fn table(source: &[u8]) -> Result<recipe_ingest::RawTable, Box<dyn Error>> {
	Ok(parse_table(
		source,
		TableRequest::new(
			Delimiter::Comma,
			HeaderMode::Present,
			IngestLimits::new(1_048_576, 64, 16, 4_096)?,
		),
	)?)
}

fn prepare(source: &[u8], targets: &[&str], train: u64, total: u64) -> Result<PreparedDataset, Box<dyn Error>> {
	let table = table(source)?;
	Ok(prepare_table(
		&table,
		&PreparationRequest::new(targets.iter().copied(), TrainFraction::new(train, total)?),
		&CategoricalEncodingModel,
	)?)
}

#[test]
fn prepares_every_declared_output_in_declaration_order_with_independent_masks() -> TestResult {
	let prepared = prepare(
		b"measure,color,numeric_target,class_target,text_target\n\
		1,red,1.5,cat,first long phrase\n\
		2,blue,2.5,dog,second long phrase\n\
		3,red,,cat,third long phrase\n\
		4,blue,4.5,dog,fourth long phrase\n\
		5,red,5.5,cat,fifth long phrase\n\
		6,green,6.5,fox,sixth long phrase\n",
		&["text_target", "numeric_target", "class_target"],
		5,
		6,
	)?;
	let references = prepare_knn_reference_set(&prepared, NonZeroU64::new(3).unwrap())?;

	assert_eq!(references.neighbors().get(), 3);
	assert_eq!(references.reference_rows(), 5);
	assert_eq!(references.reference_source_rows(), [0, 1, 2, 3, 4]);
	assert_eq!(references.feature_width(), 4);
	assert_eq!(references.reference_feature_bits().len(), 20);
	assert_eq!(
		references
			.outputs()
			.iter()
			.map(|output| output.schema().name())
			.collect::<Vec<_>>(),
		[
			b"text_target".as_slice(),
			b"numeric_target",
			b"class_target"
		]
	);

	let text = &references.outputs()[0];
	assert_eq!(text.known(), [1, 1, 1, 1, 1]);
	let KnnReferenceValues::DiscreteI32 { codes, labels } = text.values() else {
		panic!("text target was not reduced through exact discrete labels");
	};
	assert_eq!(labels.len(), 5);
	let decoded_text = codes
		.iter()
		.map(|code| text.decode_class(*code))
		.collect::<Vec<_>>();
	assert_eq!(
		decoded_text,
		[
			Some(&KnnLabelValue::Bytes(b"first long phrase".to_vec())),
			Some(&KnnLabelValue::Bytes(b"second long phrase".to_vec())),
			Some(&KnnLabelValue::Bytes(b"third long phrase".to_vec())),
			Some(&KnnLabelValue::Bytes(b"fourth long phrase".to_vec())),
			Some(&KnnLabelValue::Bytes(b"fifth long phrase".to_vec())),
		]
	);

	let numeric = &references.outputs()[1];
	assert_eq!(numeric.known(), [1, 1, 0, 1, 1]);
	assert_eq!(numeric.known_references(), 4);
	assert_eq!(
		numeric.values(),
		&KnnReferenceValues::NumericF32Bits(vec![
			1.5_f32.to_bits(),
			2.5_f32.to_bits(),
			0.0_f32.to_bits(),
			4.5_f32.to_bits(),
			5.5_f32.to_bits(),
		])
	);

	let class = &references.outputs()[2];
	assert_eq!(class.known(), [1, 1, 1, 1, 1]);
	let KnnReferenceValues::DiscreteI32 { codes, labels } = class.values() else {
		panic!("categorical target was not reduced through class codes");
	};
	assert_eq!(labels.len(), 2);
	assert_eq!(
		codes.iter()
			.map(|code| class.decode_class(*code))
			.collect::<Vec<_>>(),
		[
			Some(&KnnLabelValue::Bytes(b"cat".to_vec())),
			Some(&KnnLabelValue::Bytes(b"dog".to_vec())),
			Some(&KnnLabelValue::Bytes(b"cat".to_vec())),
			Some(&KnnLabelValue::Bytes(b"dog".to_vec())),
			Some(&KnnLabelValue::Bytes(b"cat".to_vec())),
		]
	);
	assert_eq!(
		references.operation_specs(),
		[
			KnnOutputSpec::Categorical {
				known_references: 5,
				classes: 5,
			},
			KnnOutputSpec::Numeric {
				known_references: 4,
			},
			KnnOutputSpec::Categorical {
				known_references: 5,
				classes: 2,
			},
		]
	);
	Ok(())
}

#[test]
fn temporal_and_ordinal_outputs_keep_exact_decoders() -> TestResult {
	let prepared = prepare(
		b"feature,when,rating\n\
		1,2026-01-01T00:00:00Z,low\n\
		2,2026-01-02T00:00:00Z,medium\n\
		3,2026-01-03T00:00:00Z,high\n\
		4,2026-01-04T00:00:00Z,medium\n",
		&["rating", "when"],
		3,
		4,
	)?;
	let references = prepare_knn_reference_set(&prepared, NonZeroU64::new(2).unwrap())?;

	let rating = &references.outputs()[0];
	let KnnReferenceValues::DiscreteI32 { labels, .. } = rating.values() else {
		panic!("ordinal target was not discrete");
	};
	assert_eq!(
		labels,
		&[
			KnnLabelValue::Bytes(b"low".to_vec()),
			KnnLabelValue::Bytes(b"medium".to_vec()),
			KnnLabelValue::Bytes(b"high".to_vec()),
		]
	);
	let when = &references.outputs()[1];
	let KnnReferenceValues::DiscreteI32 { labels, .. } = when.values() else {
		panic!("temporal target was not discrete");
	};
	assert_eq!(
		labels,
		&[
			KnnLabelValue::I32(0),
			KnnLabelValue::I32(86_400),
			KnnLabelValue::I32(172_800),
		]
	);
	Ok(())
}

#[test]
fn rejects_an_output_without_any_known_training_reference() -> TestResult {
	let source = b"feature,target\n1,\n2,\n3,\n4,known\n";
	let table = table(source)?;
	let inferred = infer_table_vectors(&table, &CategoricalEncodingModel)?;
	let prepared = prepare_inferred_table(
		&table,
		&inferred,
		&PreparationRequest::new(["target"], TrainFraction::new(3, 4)?),
	)?;
	let error = prepare_knn_reference_set(&prepared, NonZeroU64::new(1).unwrap()).unwrap_err();
	assert_eq!(error.kind, TrainingCompileErrorKind::InvalidTargetMatrix);
	assert!(error.detail.contains("no known reference values") || error.detail.contains("dictionary is empty"));
	Ok(())
}

#[test]
fn semantic_artifact_round_trips_every_reference_bit_as_canonical_ogdl() -> TestResult {
	let prepared = prepare(
		b"measure,color,numeric_target,class_target,text_target\n\
		1,red,1.5,cat,first long phrase\n\
		2,blue,2.5,dog,second long phrase\n\
		3,red,,cat,third long phrase\n\
		4,blue,4.5,dog,fourth long phrase\n\
		5,red,5.5,cat,fifth long phrase\n\
		6,green,6.5,fox,sixth long phrase\n",
		&["text_target", "numeric_target", "class_target"],
		5,
		6,
	)?;
	let references = prepare_knn_reference_set(&prepared, NonZeroU64::new(3).unwrap())?;
	let artifact = KnnModelArtifact::new(
		references,
		Some(DenseDataNormalization::ZScore),
		[
			DenseOperation::Normalization(DenseNormalization::Layer),
			DenseOperation::Activation(DenseActivation::Silu),
		],
	)?;

	let encoded = artifact.encode()?;
	let text = core::str::from_utf8(&encoded)?;
	assert!(text.starts_with("recipe-knn-model\tformat-version\t1\n"));
	assert!(text.contains("reference-feature-f32-bits"));
	assert!(text.contains("known-mask"));
	assert!(text.contains("value-bytes"));
	let decoded = KnnModelArtifact::decode(&encoded, KnnModelDecodeLimits::default())?;

	assert_eq!(decoded, artifact);
	assert_eq!(decoded.references(), artifact.references());
	assert_eq!(decoded.encode()?, encoded);
	Ok(())
}

#[test]
fn continuation_appends_references_retains_duplicates_and_extends_row_derived_labels() -> TestResult {
	let saved = prepare(
		b"feature,target\n\
		1,first long phrase\n\
		2,second long phrase\n\
		3,old held out phrase\n",
		&["target"],
		2,
		3,
	)?;
	let current = prepare(
		b"feature,target\n\
		2,second long phrase\n\
		4,third long phrase\n\
		5,new held out phrase\n",
		&["target"],
		2,
		3,
	)?;
	let saved = KnnModelArtifact::new(
		prepare_knn_reference_set(&saved, NonZeroU64::new(2).unwrap())?,
		None,
		[],
	)?;
	let current = KnnModelArtifact::new(
		prepare_knn_reference_set(&current, NonZeroU64::new(2).unwrap())?,
		None,
		[],
	)?;

	let continued = saved.continue_with(current)?;
	let references = continued.references();
	assert_eq!(references.reference_rows(), 4);
	assert_eq!(references.reference_source_rows(), [0, 1, 0, 1]);
	assert_eq!(
		references
			.reference_feature_bits()
			.iter()
			.copied()
			.map(f32::from_bits)
			.collect::<Vec<_>>(),
		[1.0, 2.0, 2.0, 4.0]
	);
	let output = &references.outputs()[0];
	let KnnReferenceValues::DiscreteI32 { codes, labels } = output.values() else {
		panic!("text output was not discrete");
	};
	assert_eq!(codes, &[0, 1, 1, 2]);
	assert_eq!(
		labels,
		&[
			KnnLabelValue::Bytes(b"first long phrase".to_vec()),
			KnnLabelValue::Bytes(b"second long phrase".to_vec()),
			KnnLabelValue::Bytes(b"third long phrase".to_vec()),
		]
	);
	assert_eq!(output.known_references(), 4);
	assert_eq!(
		KnnModelArtifact::decode(&continued.encode()?, KnnModelDecodeLimits::default())?,
		continued
	);
	Ok(())
}

#[test]
fn continuation_rejects_neighbor_normalization_and_schema_drift() -> TestResult {
	let prepared = prepare(
		b"feature,target\n1,alpha\n2,beta\n3,alpha\n",
		&["target"],
		2,
		3,
	)?;
	let references = prepare_knn_reference_set(&prepared, NonZeroU64::new(1).unwrap())?;
	let saved = KnnModelArtifact::new(references.clone(), None, [])?;
	let different_neighbors = KnnModelArtifact::new(
		prepare_knn_reference_set(&prepared, NonZeroU64::new(2).unwrap())?,
		None,
		[],
	)?;
	assert!(matches!(
		saved.clone().continue_with(different_neighbors),
		Err(CheckpointError::IncompatibleResume { detail }) if detail.contains("neighbor count")
	));
	let different_normalization = KnnModelArtifact::new(references, Some(DenseDataNormalization::MinMax), [])?;
	assert!(matches!(
		saved.continue_with(different_normalization),
		Err(CheckpointError::IncompatibleResume { detail }) if detail.contains("normalization")
	));
	Ok(())
}

#[test]
fn semantic_artifact_rejects_noncanonical_hex_and_bounded_resources() -> TestResult {
	let prepared = prepare(
		b"feature,target\n1,alpha\n2,beta\n3,alpha\n4,beta\n",
		&["target"],
		3,
		4,
	)?;
	let references = prepare_knn_reference_set(&prepared, NonZeroU64::new(1).unwrap())?;
	let encoded = KnnModelArtifact::new(references, None, [])?.encode()?;

	let mut uppercase_hex = encoded.clone();
	let mut cursor = 0;
	let digit = loop {
		let Some(relative_start) = uppercase_hex[cursor..]
			.windows(2)
			.position(|bytes| bytes == b"0x")
		else {
			panic!("fixture has no hexadecimal letter");
		};
		let start = cursor + relative_start + 2;
		let end = uppercase_hex[start..]
			.iter()
			.position(|byte| *byte == b'\n')
			.map_or(uppercase_hex.len(), |relative| start + relative);
		if let Some(relative) = uppercase_hex[start..end]
			.iter()
			.position(|byte| matches!(*byte, b'a'..=b'f'))
		{
			break &mut uppercase_hex[start + relative];
		}
		cursor = end;
	};
	*digit = digit.to_ascii_uppercase();
	assert_knn_decode_error(
		&uppercase_hex,
		KnnModelDecodeLimits::default(),
		CheckpointDecodeErrorKind::InvalidValue,
	);

	assert_knn_decode_error(
		&encoded,
		KnnModelDecodeLimits {
			source_bytes: encoded.len() - 1,
			..KnnModelDecodeLimits::default()
		},
		CheckpointDecodeErrorKind::LimitExceeded,
	);
	assert_knn_decode_error(
		&encoded,
		KnnModelDecodeLimits {
			total_payload_bytes: 0,
			..KnnModelDecodeLimits::default()
		},
		CheckpointDecodeErrorKind::LimitExceeded,
	);
	Ok(())
}

#[test]
fn semantic_artifact_rejects_duplicate_and_unknown_top_level_fields() -> TestResult {
	let prepared = prepare(
		b"feature,target\n1,10\n2,20\n3,30\n4,40\n",
		&["target"],
		3,
		4,
	)?;
	let references = prepare_knn_reference_set(&prepared, NonZeroU64::new(1).unwrap())?;
	let encoded = KnnModelArtifact::new(references, None, [])?.encode()?;
	let root_line = encoded.iter().position(|byte| *byte == b'\n').unwrap() + 1;

	let mut duplicate = encoded[..root_line].to_vec();
	duplicate.extend_from_slice(b"\tformat-version\n\t\t1\n");
	duplicate.extend_from_slice(&encoded[root_line..]);
	assert_knn_decode_error(
		&duplicate,
		KnnModelDecodeLimits::default(),
		CheckpointDecodeErrorKind::DuplicateField,
	);

	let mut unknown = encoded[..root_line].to_vec();
	unknown.extend_from_slice(b"\tunexpected-state\n\t\t1\n");
	unknown.extend_from_slice(&encoded[root_line..]);
	assert_knn_decode_error(
		&unknown,
		KnnModelDecodeLimits::default(),
		CheckpointDecodeErrorKind::UnknownField,
	);
	Ok(())
}

fn assert_knn_decode_error(bytes: &[u8], limits: KnnModelDecodeLimits, kind: CheckpointDecodeErrorKind) {
	let error = KnnModelArtifact::decode(bytes, limits).unwrap_err();
	let CheckpointError::Decode(error) = error else {
		panic!("expected typed KNN decode error, got {error}");
	};
	assert_eq!(error.kind(), kind);
}
