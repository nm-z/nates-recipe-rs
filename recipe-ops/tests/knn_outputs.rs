use std::collections::BTreeMap;
use std::error::Error;

use recipe_core::{AliasPermission, ByteCount, DType, KernelTemplateId, ScalarOpcode, ValueId};
use recipe_language::{CalculationGraph, Histogram, PrimitiveKind, Shape, Sort, SortDirection, Tensor};
use recipe_ops::{
	IdentityNamespace, KnnAllOutputRequest, KnnOutputRequest, KnnOutputSpec, append_knn_all_outputs,
	knn_all_output_requirements, materialize_knn_all_outputs,
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

fn request() -> Result<KnnAllOutputRequest, recipe_language::LanguageError> {
	Ok(KnnAllOutputRequest {
		query_features: tensor(1, DType::F32, &[2, 3], true, false)?,
		reference_features: tensor(2, DType::F32, &[4, 3], true, false)?,
		outputs: vec![
			KnnOutputRequest::Numeric {
				reference_values: tensor(3, DType::F32, &[4], true, false)?,
				known: tensor(4, DType::I32, &[4], true, false)?,
				predictions: tensor(5, DType::F32, &[2, 1], false, true)?,
				known_references: 3,
			},
			KnnOutputRequest::Categorical {
				reference_codes: tensor(6, DType::I32, &[4], true, false)?,
				known: tensor(7, DType::I32, &[4], true, false)?,
				predictions: tensor(8, DType::I32, &[2, 1], false, true)?,
				known_references: 4,
				classes: 3,
			},
		],
		neighbors: 3,
		tree_lanes: 4,
		identity_namespace: IdentityNamespace::new(ValueId::new(1_000), 33, KernelTemplateId::new(2_000), 32),
		workspace_limit: ByteCount::new(684),
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
fn typed_outputs_share_distances_and_keep_independent_aggregation() -> TestResult {
	let requirements = knn_all_output_requirements(
		2,
		4,
		3,
		3,
		&[
			KnnOutputSpec::Numeric {
				known_references: 3,
			},
			KnnOutputSpec::Categorical {
				known_references: 4,
				classes: 3,
			},
		],
	)?;
	assert_eq!(requirements.intermediate_values, 33);
	assert_eq!(requirements.kernels, 32);
	assert_eq!(requirements.workspace_bytes, ByteCount::new(684));
	assert_eq!(requirements.outputs[0].effective_neighbors, 3);
	assert_eq!(requirements.outputs[1].classes, Some(3));

	let materialized = materialize_knn_all_outputs(&request()?)?;
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
	assert_eq!(
		materialized
			.graph
			.nodes
			.iter()
			.filter(|node| matches!(node.kernel.kind, PrimitiveKind::Sort(_)))
			.count(),
		3
	);
	assert!(materialized.graph.nodes.iter().any(|node| {
		matches!(
			node.kernel.kind,
			PrimitiveKind::Sort(Sort {
				axis: 1,
				direction: SortDirection::Descending,
				stable: true,
				emit_indices: true,
			})
		)
	}));
	assert!(materialized.graph.nodes.iter().any(|node| {
		matches!(
			node.kernel.kind,
			PrimitiveKind::Histogram(Histogram {
				bins: 6,
				weighted: false,
				..
			})
		)
	}));
	assert!(materialized.graph.nodes.iter().any(|node| {
		matches!(&node.kernel.kind, PrimitiveKind::Elementwise(elementwise)
			if elementwise.program.instructions.iter().any(|instruction| instruction.opcode == ScalarOpcode::Require))
	}));
	Ok(())
}

#[test]
fn typed_output_knn_appends_without_redeclaring_boundaries() -> TestResult {
	let request = request()?;
	let mut tensors = vec![
		request.query_features.clone(),
		request.reference_features.clone(),
	];
	for output in &request.outputs {
		match output {
			KnnOutputRequest::Numeric {
				reference_values,
				known,
				predictions,
				..
			} => tensors.extend([reference_values.clone(), known.clone(), predictions.clone()]),
			KnnOutputRequest::Categorical {
				reference_codes,
				known,
				predictions,
				..
			} => tensors.extend([reference_codes.clone(), known.clone(), predictions.clone()]),
		}
	}
	let mut graph = CalculationGraph {
		tensors,
		nodes: Vec::new(),
	};
	let materialized = append_knn_all_outputs(&mut graph, &request)?;
	graph.validate()?;
	assert_eq!(
		graph.tensors.len(),
		8 + materialized.intermediate_values.len()
	);
	assert_eq!(graph.nodes.len(), materialized.kernels.len());
	Ok(())
}

#[test]
fn typed_output_knn_rejects_empty_semantics_and_under_reservation() -> TestResult {
	let error = knn_all_output_requirements(2, 4, 3, 3, &[]).unwrap_err();
	assert!(error.detail.contains("at least one declared output"));
	let error = knn_all_output_requirements(
		2,
		4,
		3,
		3,
		&[KnnOutputSpec::Categorical {
			known_references: 4,
			classes: 0,
		}],
	)
	.unwrap_err();
	assert!(error.detail.contains("class count"));
	let error = knn_all_output_requirements(
		1,
		16_777_217,
		1,
		16_777_217,
		&[KnnOutputSpec::Numeric {
			known_references: 16_777_217,
		}],
	)
	.unwrap_err();
	assert!(error.detail.contains("exact f32 integer domain"));
	let mut request = request()?;
	request.identity_namespace = IdentityNamespace::new(ValueId::new(1_000), 32, KernelTemplateId::new(2_000), 32);
	assert_eq!(
		materialize_knn_all_outputs(&request).unwrap_err().kind,
		recipe_ops::OperationErrorKind::IdentityNamespaceExhausted
	);
	Ok(())
}
