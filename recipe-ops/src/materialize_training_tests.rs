use std::collections::BTreeSet;
use std::error::Error;

use recipe_core::{AliasPermission, ByteCount, DType, KernelTemplateId, ScalarOpcode, ValueId};
use recipe_language::{Contraction, PrimitiveKind, Reduce, Shape, Tensor};

use crate::{
	IdentityNamespace, MaterializationRequest, MaterializedComposition, NamedTensor, OperationErrorKind,
	PreparedParameter, PreparedParameters, PrimitiveFamily, materialize_composition, operation_registry,
	remaining_composition_manifest,
};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

const MATERIALIZED: &[(&str, &str)] = &[
	("gpu_bce_with_logits", "gpu-core/src/losses.rs:145"),
	(
		"gpu_linear_backward_full_into",
		"gpu-core/src/kernels.rs:3434",
	),
	(
		"gpu_linear_backward_weights_only_into",
		"gpu-core/src/kernels.rs:3400",
	),
	("gpu_linear_f32", "gpu-core/src/nn_f32.rs:200"),
	("gpu_linear_into", "gpu-core/src/kernels.rs:2796"),
	("gpu_matvec_bias_into", "gpu-core/src/kernels.rs:3084"),
];

fn tensor(id: u64, dtype: DType, extents: &[u64], external_input: bool, external_output: bool) -> TestResult<Tensor> {
	Ok(Tensor::contiguous(
		ValueId::new(id),
		dtype,
		Shape::new(extents.to_vec())?,
		external_input,
		external_output,
	)?)
}

fn input_tensor(id: u64, extents: &[u64]) -> TestResult<Tensor> {
	tensor(id, DType::F32, extents, true, false)
}

fn output_tensor(id: u64, extents: &[u64]) -> TestResult<Tensor> {
	tensor(id, DType::F32, extents, false, true)
}

fn namespace(seed: u64) -> IdentityNamespace {
	IdentityNamespace::new(ValueId::new(seed), 100, KernelTemplateId::new(seed), 100)
}

fn tree_parameters(lanes: u64) -> PreparedParameters {
	[("tree_lanes".to_owned(), PreparedParameter::U64(lanes))]
		.into_iter()
		.collect()
}

fn families(materialized: &MaterializedComposition) -> Vec<PrimitiveFamily> {
	materialized
		.graph()
		.nodes
		.iter()
		.map(|node| match node.kernel.kind {
			PrimitiveKind::Elementwise(_) => PrimitiveFamily::Elementwise,
			PrimitiveKind::Reduce(_) => PrimitiveFamily::Reduce,
			PrimitiveKind::Scan(_) => PrimitiveFamily::Scan,
			PrimitiveKind::Contraction(_) => PrimitiveFamily::Contraction,
			PrimitiveKind::Gather(_) => PrimitiveFamily::Gather,
			PrimitiveKind::Scatter(_) => PrimitiveFamily::Scatter,
			PrimitiveKind::Histogram(_) => PrimitiveFamily::Histogram,
			PrimitiveKind::Sort(_) => PrimitiveFamily::Sort,
			PrimitiveKind::IndexMap(_) => PrimitiveFamily::IndexMap,
			PrimitiveKind::Random(_) => PrimitiveFamily::Random,
		})
		.collect()
}

fn contractions(materialized: &MaterializedComposition) -> Vec<&Contraction> {
	materialized
		.graph()
		.nodes
		.iter()
		.filter_map(|node| match &node.kernel.kind {
			PrimitiveKind::Contraction(contraction) => Some(contraction),
			_ => None,
		})
		.collect()
}

fn reductions(materialized: &MaterializedComposition) -> Vec<&Reduce> {
	materialized
		.graph()
		.nodes
		.iter()
		.filter_map(|node| match &node.kernel.kind {
			PrimitiveKind::Reduce(reduction) => Some(reduction),
			_ => None,
		})
		.collect()
}

fn assert_forbidden_aliases(materialized: &MaterializedComposition) {
	for node in &materialized.graph().nodes {
		assert_eq!(
			node.kernel.alias_rules.len(),
			node.kernel.inputs.len() * node.kernel.outputs.len()
		);
		assert!(
			node.kernel
				.alias_rules
				.iter()
				.all(|rule| rule.permission == AliasPermission::Forbidden)
		);
	}
}

#[test]
fn source_qualified_training_materializers_leave_the_remaining_manifest() -> TestResult {
	let remaining = remaining_composition_manifest()
		.into_iter()
		.map(|entry| (entry.symbol(), entry.source()))
		.collect::<BTreeSet<_>>();
	for entry in MATERIALIZED {
		assert!(!remaining.contains(entry));
	}
	Ok(())
}

#[test]
fn materializes_matrix_linear_and_specialized_matvec_with_exact_workspace() -> TestResult {
	for (index, symbol) in ["gpu_linear_into", "gpu_linear_f32"].iter().enumerate() {
		let input = input_tensor(1, &[3, 5])?;
		let weight = input_tensor(2, &[5, 7])?;
		let bias = input_tensor(3, &[7])?;
		let output = output_tensor(4, &[3, 7])?;
		let inputs = [
			NamedTensor::new("input", &input),
			NamedTensor::new("weight", &weight),
			NamedTensor::new("bias", &bias),
		];
		let outputs = [NamedTensor::new("output", &output)];
		let materialized = materialize_composition(MaterializationRequest::new(
			operation_registry().resolve_unique(symbol)?,
			&inputs,
			&outputs,
			"input",
			&PreparedParameters::new(),
			namespace(10_000 + u64::try_from(index)? * 1_000),
			ByteCount::new(84),
		))?;
		materialized.graph().validate()?;
		assert_eq!(
			families(&materialized),
			[PrimitiveFamily::Contraction, PrimitiveFamily::Elementwise]
		);
		assert_eq!(materialized.workspace().bytes(), ByteCount::new(84));
		assert_eq!(contractions(&materialized)[0].contract_axes, [(1, 0)]);
		assert_forbidden_aliases(&materialized);
	}

	let input = input_tensor(11, &[3, 5])?;
	let weight = input_tensor(12, &[5])?;
	let bias = input_tensor(13, &[1])?;
	let output = output_tensor(14, &[3])?;
	let inputs = [
		NamedTensor::new("input", &input),
		NamedTensor::new("weight", &weight),
		NamedTensor::new("bias", &bias),
	];
	let outputs = [NamedTensor::new("output", &output)];
	let materialized = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_matvec_bias_into")?,
		&inputs,
		&outputs,
		"input",
		&PreparedParameters::new(),
		namespace(20_000),
		ByteCount::new(12),
	))?;
	materialized.graph().validate()?;
	assert_eq!(
		families(&materialized),
		[PrimitiveFamily::Contraction, PrimitiveFamily::Elementwise]
	);
	assert_eq!(materialized.workspace().bytes(), ByteCount::new(12));
	assert_eq!(contractions(&materialized)[0].contract_axes, [(1, 0)]);
	assert_forbidden_aliases(&materialized);
	Ok(())
}

#[test]
fn materializes_linear_backward_contractions_and_fixed_tree_bias_sum() -> TestResult {
	let output_gradient = input_tensor(1, &[3, 7])?;
	let input = input_tensor(2, &[3, 5])?;
	let weight = input_tensor(3, &[5, 7])?;
	let input_gradient = output_tensor(4, &[3, 5])?;
	let weight_gradient = output_tensor(5, &[5, 7])?;
	let bias_gradient = output_tensor(6, &[7])?;
	let inputs = [
		NamedTensor::new("output_gradient", &output_gradient),
		NamedTensor::new("input", &input),
		NamedTensor::new("weight", &weight),
	];
	let outputs = [
		NamedTensor::new("input_gradient", &input_gradient),
		NamedTensor::new("weight_gradient", &weight_gradient),
		NamedTensor::new("bias_gradient", &bias_gradient),
	];
	let prepared = tree_parameters(64);
	let full = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_linear_backward_full_into")?,
		&inputs,
		&outputs,
		"output_gradient",
		&prepared,
		namespace(30_000),
		ByteCount::new(0),
	))?;
	full.graph().validate()?;
	assert_eq!(
		families(&full),
		[
			PrimitiveFamily::Contraction,
			PrimitiveFamily::Contraction,
			PrimitiveFamily::Reduce,
		]
	);
	assert_eq!(full.workspace().bytes(), ByteCount::new(0));
	let full_contractions = contractions(&full);
	assert_eq!(full_contractions[0].contract_axes, [(1, 1)]);
	assert_eq!(full_contractions[1].contract_axes, [(0, 0)]);
	assert_eq!(reductions(&full)[0].axes.as_slice(), [0]);
	assert_eq!(reductions(&full)[0].tree_lanes, 64);
	assert_forbidden_aliases(&full);

	let inputs = [
		NamedTensor::new("output_gradient", &output_gradient),
		NamedTensor::new("input", &input),
	];
	let outputs = [
		NamedTensor::new("weight_gradient", &weight_gradient),
		NamedTensor::new("bias_gradient", &bias_gradient),
	];
	let weights_only = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_linear_backward_weights_only_into")?,
		&inputs,
		&outputs,
		"output_gradient",
		&prepared,
		namespace(40_000),
		ByteCount::new(0),
	))?;
	weights_only.graph().validate()?;
	assert_eq!(
		families(&weights_only),
		[PrimitiveFamily::Contraction, PrimitiveFamily::Reduce]
	);
	assert_eq!(contractions(&weights_only)[0].contract_axes, [(0, 0)]);
	assert_eq!(reductions(&weights_only)[0].tree_lanes, 64);
	assert_forbidden_aliases(&weights_only);
	Ok(())
}

#[test]
fn materializes_stable_bce_loss_and_gradient_in_one_owned_scalar_program() -> TestResult {
	let logits = input_tensor(1, &[2, 3])?;
	let targets = input_tensor(2, &[2, 3])?;
	let losses = output_tensor(3, &[2, 3])?;
	let gradients = output_tensor(4, &[2, 3])?;
	let inputs = [
		NamedTensor::new("logits", &logits),
		NamedTensor::new("targets", &targets),
	];
	let outputs = [
		NamedTensor::new("losses", &losses),
		NamedTensor::new("gradients", &gradients),
	];
	let materialized = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_bce_with_logits")?,
		&inputs,
		&outputs,
		"logits",
		&PreparedParameters::new(),
		namespace(50_000),
		ByteCount::new(0),
	))?;
	materialized.graph().validate()?;
	assert_eq!(families(&materialized), [PrimitiveFamily::Elementwise]);
	assert_eq!(materialized.workspace().bytes(), ByteCount::new(0));
	let PrimitiveKind::Elementwise(elementwise) = &materialized.graph().nodes[0].kernel.kind else {
		panic!("BCE must materialize as one elementwise program");
	};
	assert_eq!(elementwise.program.inputs.len(), 2);
	assert_eq!(elementwise.program.outputs.len(), 2);
	let opcodes = elementwise
		.program
		.instructions
		.iter()
		.map(|instruction| instruction.opcode)
		.collect::<Vec<_>>();
	assert!(opcodes.contains(&ScalarOpcode::Select));
	assert!(opcodes.contains(&ScalarOpcode::Absolute));
	assert!(opcodes.contains(&ScalarOpcode::Require));
	assert_forbidden_aliases(&materialized);
	Ok(())
}

#[test]
fn rejects_non_exact_training_abis_shapes_parameters_and_workspace() -> TestResult {
	let input = input_tensor(1, &[3, 5])?;
	let bad_weight = input_tensor(2, &[4, 7])?;
	let bias = input_tensor(3, &[7])?;
	let output = output_tensor(4, &[3, 7])?;
	let inputs = [
		NamedTensor::new("input", &input),
		NamedTensor::new("weight", &bad_weight),
		NamedTensor::new("bias", &bias),
	];
	let outputs = [NamedTensor::new("output", &output)];
	let error = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_linear_into")?,
		&inputs,
		&outputs,
		"input",
		&PreparedParameters::new(),
		namespace(60_000),
		ByteCount::new(84),
	))
	.expect_err("mismatched contraction extents must fail");
	assert_eq!(
		error.kind,
		OperationErrorKind::InvalidMaterializationRequest
	);

	let weight = input_tensor(12, &[5, 7])?;
	let inputs = [
		NamedTensor::new("input", &input),
		NamedTensor::new("weight", &weight),
		NamedTensor::new("bias", &bias),
	];
	let extra_parameters = [("unexpected".to_owned(), PreparedParameter::Bool(true))]
		.into_iter()
		.collect();
	let error = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_linear_into")?,
		&inputs,
		&outputs,
		"input",
		&extra_parameters,
		namespace(61_000),
		ByteCount::new(84),
	))
	.expect_err("an extra parameter must fail the exact ABI");
	assert_eq!(
		error.kind,
		OperationErrorKind::InvalidMaterializationRequest
	);

	let error = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_linear_into")?,
		&inputs,
		&outputs,
		"input",
		&PreparedParameters::new(),
		namespace(62_000),
		ByteCount::new(83),
	))
	.expect_err("forward contraction scratch must be reserved exactly");
	assert_eq!(error.kind, OperationErrorKind::WorkspaceLimitExceeded);

	let targets = input_tensor(20, &[2, 3])?;
	let logits = input_tensor(21, &[2, 3])?;
	let losses = output_tensor(22, &[2, 3])?;
	let bad_gradients = output_tensor(23, &[3, 2])?;
	let inputs = [
		NamedTensor::new("logits", &logits),
		NamedTensor::new("targets", &targets),
	];
	let outputs = [
		NamedTensor::new("losses", &losses),
		NamedTensor::new("gradients", &bad_gradients),
	];
	let error = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_bce_with_logits")?,
		&inputs,
		&outputs,
		"logits",
		&PreparedParameters::new(),
		namespace(63_000),
		ByteCount::new(0),
	))
	.expect_err("BCE output shapes must exactly match logits");
	assert_eq!(error.kind, OperationErrorKind::UnsupportedConcreteShape);
	Ok(())
}
