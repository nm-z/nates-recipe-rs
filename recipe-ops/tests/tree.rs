use std::collections::BTreeMap;
use std::error::Error;

use recipe_core::{AliasPermission, ByteCount, DType, KernelTemplateId, ValueId};
use recipe_language::{CalculationGraph, Shape, Tensor};
use recipe_ops::{
	IdentityNamespace, TreeEnsembleInferenceRequest, append_tree_ensemble_inference,
	materialize_tree_ensemble_inference, tree_ensemble_inference_requirements,
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
fn saved_tree_ensemble_traversal_is_a_lowerable_static_graph() -> TestResult {
	let requirements = tree_ensemble_inference_requirements(2, 3, 2, 2, 2)?;
	assert_eq!(requirements.internal_nodes_per_tree, 3);
	assert_eq!(requirements.leaves_per_tree, 4);
	assert_eq!(requirements.intermediate_values, 19);
	assert_eq!(requirements.kernels, 20);
	assert_eq!(requirements.workspace_bytes, ByteCount::new(328));
	let request = TreeEnsembleInferenceRequest {
		features: tensor(1, DType::F32, &[6], true, false)?,
		split_features: tensor(2, DType::I32, &[6], true, false)?,
		split_thresholds: tensor(3, DType::F32, &[6], true, false)?,
		leaf_values: tensor(4, DType::F32, &[16], true, false)?,
		predictions: tensor(5, DType::F32, &[2, 2], false, true)?,
		rows: 2,
		feature_width: 3,
		trees: 2,
		depth: 2,
		outputs: 2,
		scale: 0.5,
		tree_lanes: 2,
		identity_namespace: IdentityNamespace::new(
			ValueId::new(1_000),
			requirements.intermediate_values,
			KernelTemplateId::new(2_000),
			requirements.kernels,
		),
		workspace_limit: requirements.workspace_bytes,
	};
	let materialized = materialize_tree_ensemble_inference(&request)?;
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
	Ok(())
}

#[test]
fn appending_tree_traversal_preserves_caller_boundaries() -> TestResult {
	let requirements = tree_ensemble_inference_requirements(2, 3, 2, 2, 2)?;
	let request = TreeEnsembleInferenceRequest {
		features: tensor(21, DType::F32, &[6], true, false)?,
		split_features: tensor(22, DType::I32, &[6], true, false)?,
		split_thresholds: tensor(23, DType::F32, &[6], true, false)?,
		leaf_values: tensor(24, DType::F32, &[16], true, false)?,
		predictions: tensor(25, DType::F32, &[2, 2], false, true)?,
		rows: 2,
		feature_width: 3,
		trees: 2,
		depth: 2,
		outputs: 2,
		scale: 0.5,
		tree_lanes: 2,
		identity_namespace: IdentityNamespace::new(
			ValueId::new(5_000),
			requirements.intermediate_values,
			KernelTemplateId::new(6_000),
			requirements.kernels,
		),
		workspace_limit: requirements.workspace_bytes,
	};
	let mut graph = CalculationGraph {
		tensors: vec![
			request.features.clone(),
			request.split_features.clone(),
			request.split_thresholds.clone(),
			request.leaf_values.clone(),
			request.predictions.clone(),
		],
		nodes: Vec::new(),
	};
	let materialized = append_tree_ensemble_inference(&mut graph, &request)?;
	graph.validate()?;
	assert_eq!(
		graph.tensors.len(),
		5 + materialized.intermediate_values.len()
	);
	assert_eq!(graph.nodes.len(), materialized.kernels.len());
	Ok(())
}

#[test]
fn tree_traversal_rejects_unindexable_or_under_reserved_shapes() -> TestResult {
	let error = tree_ensemble_inference_requirements(1, 1, 1, 31, 1).unwrap_err();
	assert!(error.detail.contains("1..=30"));
	let requirements = tree_ensemble_inference_requirements(1, 2, 1, 1, 1)?;
	let mut request = TreeEnsembleInferenceRequest {
		features: tensor(11, DType::F32, &[2], true, false)?,
		split_features: tensor(12, DType::I32, &[1], true, false)?,
		split_thresholds: tensor(13, DType::F32, &[1], true, false)?,
		leaf_values: tensor(14, DType::F32, &[2], true, false)?,
		predictions: tensor(15, DType::F32, &[1, 1], false, true)?,
		rows: 1,
		feature_width: 2,
		trees: 1,
		depth: 1,
		outputs: 1,
		scale: 1.0,
		tree_lanes: 1,
		identity_namespace: IdentityNamespace::new(
			ValueId::new(3_000),
			requirements.intermediate_values - 1,
			KernelTemplateId::new(4_000),
			requirements.kernels,
		),
		workspace_limit: requirements.workspace_bytes,
	};
	assert!(
		materialize_tree_ensemble_inference(&request)
			.unwrap_err()
			.detail
			.contains("intermediate identities")
	);
	request.identity_namespace = IdentityNamespace::new(
		ValueId::new(3_000),
		requirements.intermediate_values,
		KernelTemplateId::new(4_000),
		requirements.kernels,
	);
	request.scale = f32::NAN;
	assert!(
		materialize_tree_ensemble_inference(&request)
			.unwrap_err()
			.detail
			.contains("scale must be finite")
	);
	Ok(())
}
