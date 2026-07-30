use std::collections::BTreeMap;
use std::error::Error;

use recipe_core::{AliasPermission, ByteCount, DType, KernelTemplateId, ValueId};
use recipe_language::{PrimitiveKind, ReduceOperator, ReduceResult, Shape, Tensor};
use recipe_ops::{
	IdentityNamespace, KMeansInitializationRequest, KMeansLloydRequest, materialize_kmeans_initialization,
	materialize_kmeans_lloyd_step,
};
use recipe_primitives::{LoweredProgram, LoweringHardware, LoweringResult};

type TestResult = Result<(), Box<dyn Error>>;

fn tensor(
	id: u64,
	extents: &[u64],
	external_input: bool,
	external_output: bool,
) -> Result<Tensor, recipe_language::LanguageError> {
	Tensor::contiguous(
		ValueId::new(id),
		DType::F32,
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

fn lower_every_node(graph: &recipe_language::CalculationGraph) -> TestResult {
	graph.validate()?;
	let tensors = graph
		.tensors
		.iter()
		.map(|tensor| (tensor.id, tensor))
		.collect::<BTreeMap<_, _>>();
	for node in &graph.nodes {
		let lowered = lower(&node.kernel, &tensors)?;
		lowered.validate().map_err(|errors| {
			errors.into_iter()
				.map(|error| error.to_string())
				.collect::<Vec<_>>()
				.join("; ")
		})?;
	}
	Ok(())
}

#[test]
fn deterministic_initialization_is_a_lowerable_gpu_graph() -> TestResult {
	let materialized = materialize_kmeans_initialization(&KMeansInitializationRequest {
		points: tensor(1, &[3, 2], true, false)?,
		centroids: tensor(2, &[5, 2], false, true)?,
		identity_namespace: IdentityNamespace::new(ValueId::new(1_000), 1, KernelTemplateId::new(2_000), 2),
		workspace_limit: ByteCount::new(20),
	})?;
	lower_every_node(&materialized.graph)
}

#[test]
fn deterministic_lloyd_step_is_a_lowerable_gpu_graph() -> TestResult {
	let materialized = materialize_kmeans_lloyd_step(&KMeansLloydRequest {
		points: tensor(1, &[4, 2], true, false)?,
		prior_centroids: tensor(2, &[2, 2], true, false)?,
		updated_centroids: tensor(3, &[2, 2], false, true)?,
		distances: tensor(4, &[4, 2], false, true)?,
		tree_lanes: 4,
		identity_namespace: IdentityNamespace::new(ValueId::new(1_000), 16, KernelTemplateId::new(2_000), 18),
		workspace_limit: ByteCount::new(344),
	})?;
	lower_every_node(&materialized.graph)?;

	assert!(materialized.graph.nodes.iter().any(|node| {
		matches!(
			node.kernel.kind,
			PrimitiveKind::Reduce(recipe_language::Reduce {
				operator: ReduceOperator::Minimum,
				result: ReduceResult::Index,
				..
			})
		)
	}));
	assert_eq!(
		materialized
			.graph
			.nodes
			.iter()
			.flat_map(|node| node.kernel.alias_rules.iter())
			.filter(|rule| rule.permission == AliasPermission::MustAliasExact)
			.count(),
		1
	);
	Ok(())
}
