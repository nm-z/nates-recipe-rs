use std::error::Error;

use recipe_ingest::{
	CategoricalEncodingModel, Delimiter, HeaderMode, IngestLimits, PreparationRequest, PreparedDataset, TableRequest,
	TrainFraction, parse_table, prepare_table,
};
use recipe_training::bayes::{
	BayesianDependency, BayesianNodeSource, BayesianSchemaErrorKind, BayesianSchemaPathSegment,
	resolve_bayesian_schema,
};

type TestResult = Result<(), Box<dyn Error>>;

fn dataset() -> Result<PreparedDataset, Box<dyn Error>> {
	let table = parse_table(
		b"income,housing,loan,approval,unused\n\
		50000,rent,no,no,1\n\
		72000,own,yes,yes,2\n\
		61000,rent,yes,yes,3\n\
		43000,own,no,no,4\n",
		TableRequest::new(
			Delimiter::Comma,
			HeaderMode::Present,
			IngestLimits::new(4096, 16, 16, 128)?,
		),
	)?;
	Ok(prepare_table(
		&table,
		&PreparationRequest::new(["approval"], TrainFraction::new(3, 4)?),
		&CategoricalEncodingModel,
	)?)
}

fn names(
	schema: &recipe_training::bayes::ResolvedBayesianSchema,
	ids: &[recipe_training::bayes::BayesianNodeId],
) -> Vec<Vec<u8>> {
	ids.iter()
		.map(|id| {
			schema.node(*id)
				.expect("resolved node exists")
				.name()
				.to_vec()
		})
		.collect()
}

#[test]
fn preserves_declaration_parent_and_typed_vector_schema() -> TestResult {
	let dataset = dataset()?;
	let declarations = [
		BayesianDependency::new("approval", ["loan", "market"]),
		BayesianDependency::new("loan", ["housing", "income"]),
	];
	let schema = resolve_bayesian_schema(&dataset, &declarations)?;

	assert_eq!(
		schema.declarations()
			.iter()
			.map(|declaration| schema.node(declaration.child()).unwrap().name().to_vec())
			.collect::<Vec<_>>(),
		[b"approval".to_vec(), b"loan".to_vec()]
	);
	assert_eq!(
		names(&schema, schema.declarations()[0].parents()),
		[b"loan".to_vec(), b"market".to_vec()]
	);
	assert_eq!(
		names(&schema, schema.declarations()[1].parents()),
		[b"housing".to_vec(), b"income".to_vec()]
	);
	for vector in dataset.vectors() {
		let node = schema.node(schema.node_id(vector.name()).unwrap()).unwrap();
		assert_eq!(node.observed_schema(), Some(&vector.schema()));
	}
	let market = schema.node(schema.node_id("market").unwrap()).unwrap();
	assert_eq!(market.source(), &BayesianNodeSource::LatentRoot);
	assert!(market.is_latent_root());
	assert!(schema.node_id("unused").is_some());
	Ok(())
}

#[test]
fn topological_order_is_canonical_and_separate_from_declaration_order() -> TestResult {
	let dataset = dataset()?;
	let forward = resolve_bayesian_schema(
		&dataset,
		&[
			BayesianDependency::new("approval", ["loan", "market"]),
			BayesianDependency::new("loan", ["housing", "income"]),
		],
	)?;
	let dependency_first = resolve_bayesian_schema(
		&dataset,
		&[
			BayesianDependency::new("loan", ["housing", "income"]),
			BayesianDependency::new("approval", ["loan", "market"]),
		],
	)?;

	assert_ne!(forward.declarations(), dependency_first.declarations());
	assert_eq!(
		names(&forward, forward.execution_order()),
		names(&dependency_first, dependency_first.execution_order())
	);
	assert_eq!(
		names(&forward, forward.execution_order()),
		[
			b"housing".to_vec(),
			b"income".to_vec(),
			b"loan".to_vec(),
			b"market".to_vec(),
			b"approval".to_vec(),
			b"unused".to_vec(),
		]
	);
	Ok(())
}

#[test]
fn malformed_declarations_report_typed_hierarchical_paths() -> TestResult {
	let dataset = dataset()?;
	let cases = [
		(
			vec![BayesianDependency::new("", ["income"])],
			BayesianSchemaErrorKind::EmptyName,
			vec![
				BayesianSchemaPathSegment::Declarations,
				BayesianSchemaPathSegment::Declaration(0),
				BayesianSchemaPathSegment::Child,
			],
		),
		(
			vec![BayesianDependency::new("loan", ["income", ""])],
			BayesianSchemaErrorKind::EmptyName,
			vec![
				BayesianSchemaPathSegment::Declarations,
				BayesianSchemaPathSegment::Declaration(0),
				BayesianSchemaPathSegment::Parents,
				BayesianSchemaPathSegment::Parent(1),
			],
		),
		(
			vec![
				BayesianDependency::new("loan", ["income"]),
				BayesianDependency::new("loan", ["housing"]),
			],
			BayesianSchemaErrorKind::DuplicateChild,
			vec![
				BayesianSchemaPathSegment::Declarations,
				BayesianSchemaPathSegment::Declaration(1),
				BayesianSchemaPathSegment::Child,
			],
		),
		(
			vec![BayesianDependency::new("loan", ["income", "income"])],
			BayesianSchemaErrorKind::DuplicateParent,
			vec![
				BayesianSchemaPathSegment::Declarations,
				BayesianSchemaPathSegment::Declaration(0),
				BayesianSchemaPathSegment::Parents,
				BayesianSchemaPathSegment::Parent(1),
			],
		),
		(
			vec![BayesianDependency::new("loan", ["loan"])],
			BayesianSchemaErrorKind::SelfDependency,
			vec![
				BayesianSchemaPathSegment::Declarations,
				BayesianSchemaPathSegment::Declaration(0),
				BayesianSchemaPathSegment::Parents,
				BayesianSchemaPathSegment::Parent(0),
			],
		),
	];
	for (dependencies, kind, path) in cases {
		let error = resolve_bayesian_schema(&dataset, &dependencies).unwrap_err();
		assert_eq!(error.kind(), kind);
		assert_eq!(error.path(), path);
		assert!(error.to_string().contains("declarations"));
	}
	Ok(())
}

#[test]
fn absent_child_with_observed_and_absent_parents_is_a_latent_conditional() -> TestResult {
	let dataset = dataset()?;
	let schema = resolve_bayesian_schema(
		&dataset,
		&[
			BayesianDependency::new("phantom", ["income", "hidden"]),
			BayesianDependency::new("approval", ["phantom"]),
		],
	)?;
	let phantom = schema.node(schema.node_id("phantom").unwrap()).unwrap();
	let hidden = schema.node(schema.node_id("hidden").unwrap()).unwrap();
	assert_eq!(phantom.source(), &BayesianNodeSource::LatentConditional);
	assert!(phantom.is_latent());
	assert!(!phantom.is_latent_root());
	assert_eq!(hidden.source(), &BayesianNodeSource::LatentRoot);
	assert!(hidden.is_latent_root());
	let order = names(&schema, schema.execution_order());
	let position = |name: &[u8]| {
		order.iter()
			.position(|candidate| candidate == name)
			.unwrap()
	};
	assert!(position(b"income") < position(b"phantom"));
	assert!(position(b"hidden") < position(b"phantom"));
	assert!(position(b"phantom") < position(b"approval"));
	Ok(())
}

#[test]
fn cycle_reports_the_execution_order_hierarchy() -> TestResult {
	let dataset = dataset()?;
	let error = resolve_bayesian_schema(
		&dataset,
		&[
			BayesianDependency::new("loan", ["approval"]),
			BayesianDependency::new("approval", ["loan"]),
		],
	)
	.unwrap_err();
	assert_eq!(error.kind(), BayesianSchemaErrorKind::Cycle);
	assert_eq!(
		error.path(),
		[
			BayesianSchemaPathSegment::Graph,
			BayesianSchemaPathSegment::ExecutionOrder,
		]
	);
	assert!(error.detail().contains("approval"));
	assert!(error.detail().contains("loan"));
	assert!(error.to_string().contains("graph.execution-order"));
	Ok(())
}

#[test]
fn empty_declaration_list_keeps_every_observed_vector_as_a_root() -> TestResult {
	let dataset = dataset()?;
	let schema = resolve_bayesian_schema(&dataset, &[])?;
	assert_eq!(schema.nodes().len(), dataset.vectors().len());
	assert_eq!(schema.execution_order().len(), dataset.vectors().len());
	assert!(schema.nodes().iter().all(|node| !node.is_latent_root()));
	Ok(())
}
