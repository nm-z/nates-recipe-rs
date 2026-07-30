use std::error::Error;

use recipe_core::DType;
use recipe_ingest::{
	CategoricalEncodingModel, Delimiter, HeaderMode, IngestLimits, PreparationRequest, RawTable, TableRequest,
	TrainFraction, parse_table, prepare_table,
};
use recipe_training::{
	BayesModelArtifact, BayesModelDecodeLimits, BayesianDependency, InferenceInputRole, InferencePredictionKind,
	InferenceTask, compile_prepared_bayes_inference, prepare_bayes_inference_table,
	prepare_categorical_bayesian_reference_set, prepare_categorical_bayesian_reference_sets,
};

type TestResult = Result<(), Box<dyn Error>>;

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

fn artifact() -> Result<BayesModelArtifact, Box<dyn Error>> {
	let source = table(b"weather,wind,play\n\
		sun,breeze,otter\n\
		sun,gale,falcon\n\
		rain,breeze,otter\n\
		rain,gale,falcon\n\
		sun,breeze,otter\n")?;
	let prepared = prepare_table(
		&source,
		&PreparationRequest::new(["play"], TrainFraction::new(4, 5)?),
		&CategoricalEncodingModel,
	)?;
	let references = prepare_categorical_bayesian_reference_set(
		&prepared,
		&[BayesianDependency::new("play", ["weather", "wind"])],
	)?;
	Ok(BayesModelArtifact::new(references)?)
}

#[test]
fn saves_exact_observations_and_round_trips_canonical_ogdl() -> TestResult {
	let artifact = artifact()?;
	let references = artifact.references();
	assert_eq!(references.reference_rows(), 4);
	assert_eq!(
		references
			.parents()
			.iter()
			.map(|parent| parent.name())
			.collect::<Vec<_>>(),
		[b"weather".as_slice(), b"wind"]
	);
	assert_eq!(references.parent_cardinalities(), [3, 3]);
	assert_eq!(references.parent_multipliers(), [3, 1]);
	assert_eq!(references.parent_configurations(), 9);
	assert_eq!(references.parent_codes().len(), 8);
	assert_eq!(references.child_codes().len(), 4);

	let encoded = artifact.encode()?;
	let text = core::str::from_utf8(&encoded)?;
	assert!(text.starts_with("recipe-bayes-model\tformat-version\t1\n"));
	assert!(text.contains("reference-parent-codes"));
	assert!(text.contains("reference-child-codes"));
	let decoded = BayesModelArtifact::decode(&encoded, BayesModelDecodeLimits::default())?;
	assert_eq!(decoded, artifact);
	assert_eq!(decoded.encode()?, encoded);
	Ok(())
}

#[test]
fn repeated_observed_outputs_reject_a_target_child_as_an_inference_parent() -> TestResult {
	let source = table(b"weather,play,travel\n\
		  sun,otter,walk\n\
		  sun,falcon,drive\n\
		  rain,otter,drive\n\
		  rain,falcon,stay\n")?;
	let prepared = prepare_table(
		&source,
		&PreparationRequest::new(["play", "travel"], TrainFraction::new(3, 4)?),
		&CategoricalEncodingModel,
	)?;
	let error = prepare_categorical_bayesian_reference_sets(
		&prepared,
		&[
			BayesianDependency::new("play", ["weather"]),
			BayesianDependency::new("travel", ["play"]),
		],
	)
	.unwrap_err();
	assert!(error.detail.contains("parent to be an inference feature"));
	Ok(())
}

#[test]
fn compiles_reordered_and_unseen_parent_rows_into_native_posterior() -> TestResult {
	let artifact = artifact()?;
	let queries = table(b"wind,unused,weather\n\
		breeze,x,sun\n\
		storm,y,cloud\n")?;
	let prepared = prepare_bayes_inference_table(artifact, &queries)?;
	let compiled = compile_prepared_bayes_inference(&prepared)?;

	compiled.graph().validate()?;
	assert_eq!(compiled.rows(), 2);
	assert_eq!(compiled.output().dtype(), DType::F32);
	assert_eq!(compiled.output().shape().extents(), [2, 2]);
	assert_eq!(
		compiled.output().kind(),
		InferencePredictionKind::BayesProbabilities
	);
	assert_eq!(
		compiled.task(),
		InferenceTask::BayesProbabilities { width: 2 }
	);
	assert_eq!(
		compiled
			.external_inputs()
			.iter()
			.map(|input| input.role())
			.collect::<Vec<_>>(),
		[
			InferenceInputRole::BayesReferenceParents { conditional: 0 },
			InferenceInputRole::BayesReferenceChild { conditional: 0 },
			InferenceInputRole::BayesQueryParents { conditional: 0 },
			InferenceInputRole::BayesParentMultipliers { conditional: 0 },
			InferenceInputRole::BayesParentCardinalities { conditional: 0 },
		]
	);
	let query = compiled
		.external_inputs()
		.iter()
		.find(|input| input.role() == InferenceInputRole::BayesQueryParents { conditional: 0 })
		.expect("query parent boundary exists");
	assert_eq!(query.shape().extents(), [2, 2]);
	assert_eq!(
		query.bytes()
			.chunks_exact(4)
			.map(|bytes| i32::from_le_bytes(bytes.try_into().unwrap()))
			.collect::<Vec<_>>(),
		[1, 0, 2, 2]
	);
	Ok(())
}

#[test]
fn repeated_conditionals_round_trip_and_compile_adjacent_probability_blocks() -> TestResult {
	let source = table(b"weather,wind,play,travel\n\
		  sun,breeze,otter,walk\n\
		  sun,gale,falcon,drive\n\
		  rain,breeze,otter,drive\n\
		  rain,gale,falcon,stay\n\
		  sun,breeze,otter,walk\n")?;
	let prepared = prepare_table(
		&source,
		&PreparationRequest::new(["play", "travel"], TrainFraction::new(4, 5)?),
		&CategoricalEncodingModel,
	)?;
	let references = prepare_categorical_bayesian_reference_sets(
		&prepared,
		&[
			BayesianDependency::new("play", ["weather", "wind"]),
			BayesianDependency::new("travel", ["weather"]),
		],
	)?;
	let artifact = BayesModelArtifact::from_conditionals(references)?;
	assert_eq!(artifact.format_version(), 2);
	assert_eq!(artifact.conditionals().len(), 2);
	let encoded = artifact.encode()?;
	let text = core::str::from_utf8(&encoded)?;
	assert!(text.starts_with("recipe-bayes-model\tformat-version\t2\n"));
	assert!(text.contains("\n\tconditionals\tconditional\t"));
	let decoded = BayesModelArtifact::decode(&encoded, BayesModelDecodeLimits::default())?;
	assert_eq!(decoded, artifact);
	assert_eq!(decoded.encode()?, encoded);
	let continued = artifact.clone().continue_with(artifact.clone())?;
	assert!(
		continued
			.conditionals()
			.iter()
			.all(|conditional| conditional.reference_rows() == 8)
	);

	let queries = table(b"wind,unused,weather\n\
		  breeze,x,sun\n\
		  storm,y,cloud\n")?;
	let prepared = prepare_bayes_inference_table(artifact, &queries)?;
	assert_eq!(prepared.data().features().len(), 2);
	let compiled = compile_prepared_bayes_inference(&prepared)?;
	compiled.graph().validate()?;
	assert_eq!(compiled.rows(), 2);
	assert_eq!(compiled.output().shape().extents(), [2, 5]);
	assert_eq!(
		compiled.output().kind(),
		InferencePredictionKind::BayesProbabilities
	);
	assert_eq!(
		compiled.task(),
		InferenceTask::BayesProbabilities { width: 5 }
	);
	assert_eq!(
		compiled
			.graph()
			.tensors
			.iter()
			.filter(|tensor| tensor.external_output)
			.count(),
		1
	);
	for conditional in 0..2 {
		assert!(
			compiled
				.external_inputs()
				.iter()
				.any(|input| { input.role() == InferenceInputRole::BayesReferenceChild { conditional } })
		);
	}
	assert!(
		compiled
			.external_inputs()
			.iter()
			.any(|input| { input.role() == InferenceInputRole::BayesConcatenationSelectLeft { join: 1 } })
	);
	Ok(())
}

#[test]
fn rejects_empty_query_data_before_graph_materialization() -> TestResult {
	let prepared = prepare_bayes_inference_table(artifact()?, &table(b"weather,wind\n")?)?;
	let error = compile_prepared_bayes_inference(&prepared).unwrap_err();
	assert!(error.detail().contains("at least one query row"));
	Ok(())
}
