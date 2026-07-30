use std::collections::BTreeMap;
use std::error::Error;

use recipe_core::{AliasPermission, ByteCount, DType, KernelTemplateId, ScalarOpcode, ValueId};
use recipe_language::{CalculationGraph, Histogram, PrimitiveKind, Shape, Tensor};
use recipe_ops::{
	CategoricalBayesInferenceRequest, IdentityNamespace, append_categorical_bayes_inference,
	categorical_bayes_inference_requirements, materialize_categorical_bayes_inference,
};
use recipe_primitives::{LoweredProgram, LoweringHardware, LoweringResult};

type TestResult = Result<(), Box<dyn Error>>;

fn tensor(
	id: u64,
	dtype: DType,
	extents: &[u64],
	external_input: bool,
	external_output: bool,
) -> Result<Tensor, recipe_language::LanguageError> {
	Tensor::contiguous(
		ValueId::new(id),
		dtype,
		Shape::new(extents.to_vec())?,
		external_input,
		external_output,
	)
}

fn request() -> Result<CategoricalBayesInferenceRequest, recipe_language::LanguageError> {
	Ok(CategoricalBayesInferenceRequest {
		reference_parent_codes: tensor(1, DType::I32, &[4, 2], true, false)?,
		reference_child_codes: tensor(2, DType::I32, &[4], true, false)?,
		query_parent_codes: tensor(3, DType::I32, &[2, 2], true, false)?,
		parent_multipliers: tensor(4, DType::I32, &[2], true, false)?,
		parent_cardinalities: tensor(5, DType::I32, &[2], true, false)?,
		probabilities: tensor(6, DType::F32, &[2, 3], false, true)?,
		reference_rows: 4,
		query_rows: 2,
		parent_count: 2,
		parent_configurations: 12,
		child_classes: 3,
		smoothing: 1.0,
		tree_lanes: 4,
		identity_namespace: IdentityNamespace::new(ValueId::new(1_000), 10, KernelTemplateId::new(2_000), 11),
		workspace_limit: ByteCount::new(300),
	})
}

fn lower(
	kernel: &recipe_language::PrimitiveKernel,
	tensors: &BTreeMap<ValueId, &Tensor>,
) -> LoweringResult<LoweredProgram> {
	recipe_primitives::lower(
		kernel,
		tensors,
		LoweringHardware {
			subgroup_lanes: 32,
			maximum_workgroup_lanes: 256,
			maximum_shared_memory_per_workgroup: ByteCount::new(64 * 1024),
		},
	)
}

#[test]
fn categorical_posterior_is_one_lowerable_static_graph() -> TestResult {
	let requirements = categorical_bayes_inference_requirements(4, 2, 2, 12, 3)?;
	assert_eq!(requirements.intermediate_values, 10);
	assert_eq!(requirements.kernels, 11);
	assert_eq!(requirements.workspace_bytes, ByteCount::new(300));

	let materialized = materialize_categorical_bayes_inference(&request()?)?;
	materialized.graph.validate()?;
	let tensors = materialized
		.graph
		.tensors
		.iter()
		.map(|tensor| (tensor.id, tensor))
		.collect::<BTreeMap<_, _>>();
	for node in &materialized.graph.nodes {
		let lowered = lower(&node.kernel, &tensors)?;
		lowered.validate().map_err(|errors| {
			errors.into_iter()
				.map(|error| error.to_string())
				.collect::<Vec<_>>()
				.join("; ")
		})?;
		assert!(
			node.kernel
				.alias_rules
				.iter()
				.all(|rule| rule.permission == AliasPermission::Forbidden)
		);
	}
	assert!(materialized.graph.nodes.iter().any(|node| {
		matches!(
			node.kernel.kind,
			PrimitiveKind::Histogram(Histogram {
				bins: 36,
				weighted: false,
				..
			})
		)
	}));
	assert!(materialized.graph.nodes.iter().any(|node| {
		matches!(&node.kernel.kind, PrimitiveKind::Elementwise(elementwise)
			if elementwise.program.instructions.iter().any(|instruction| instruction.opcode == ScalarOpcode::Divide))
	}));
	Ok(())
}

#[test]
fn categorical_posterior_appends_without_redeclaring_boundaries() -> TestResult {
	let request = request()?;
	let mut graph = CalculationGraph {
		tensors: vec![
			request.reference_parent_codes.clone(),
			request.reference_child_codes.clone(),
			request.query_parent_codes.clone(),
			request.parent_multipliers.clone(),
			request.parent_cardinalities.clone(),
			request.probabilities.clone(),
		],
		nodes: Vec::new(),
	};
	let materialized = append_categorical_bayes_inference(&mut graph, &request)?;
	graph.validate()?;
	assert_eq!(
		graph.tensors.len(),
		6 + materialized.intermediate_values.len()
	);
	assert_eq!(graph.nodes.len(), materialized.kernels.len());
	Ok(())
}

#[test]
fn categorical_posterior_rejects_unbounded_state_or_invalid_smoothing() -> TestResult {
	let error = categorical_bayes_inference_requirements(1, 1, 2, i32::MAX as u64, 2).unwrap_err();
	assert!(error.detail.contains("state space"));
	let mut request = request()?;
	request.smoothing = 0.0;
	assert!(
		materialize_categorical_bayes_inference(&request)
			.unwrap_err()
			.detail
			.contains("positive")
	);
	Ok(())
}
