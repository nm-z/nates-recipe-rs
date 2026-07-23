use std::collections::BTreeSet;
use std::error::Error;

use recipe_core::{ByteCount, DType, KernelTemplateId, ValueId};
use recipe_language::{CalculationGraph, Shape, Tensor};

use crate::{
	IdentityNamespace, MaterializationRequest, MaterializedComposition, NamedTensor, OperationErrorKind,
	PreparedParameter, PreparedParameters, materialize_composition, operation_registry, validate_identity_namespaces,
};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

fn input_tensor(id: u64, extents: &[u64]) -> TestResult<Tensor> {
	Ok(Tensor::contiguous(
		ValueId::new(id),
		DType::F32,
		Shape::new(extents.to_vec())?,
		true,
		false,
	)?)
}

fn output_tensor(id: u64, extents: &[u64]) -> TestResult<Tensor> {
	Ok(Tensor::contiguous(
		ValueId::new(id),
		DType::F32,
		Shape::new(extents.to_vec())?,
		false,
		true,
	)?)
}

fn namespace(seed: u64) -> IdentityNamespace {
	IdentityNamespace::new(
		ValueId::new(seed),
		1_000,
		KernelTemplateId::new(seed),
		1_000,
	)
}

fn parameters(values: &[(&str, PreparedParameter)]) -> PreparedParameters {
	values.iter()
		.map(|(name, value)| ((*name).to_owned(), *value))
		.collect()
}

fn f32_parameter(value: f32) -> PreparedParameter {
	PreparedParameter::F32Bits(value.to_bits())
}

fn assert_graph(materialized: &MaterializedComposition, workspace: u64) -> TestResult {
	materialized.graph().validate()?;
	assert_eq!(materialized.workspace().bytes(), ByteCount::new(workspace));
	assert_eq!(
		materialized.stages().len(),
		materialized.resolved().steps().len()
	);
	Ok(())
}

fn momentum_fragment(base: u64, identities: IdentityNamespace) -> TestResult<MaterializedComposition> {
	let weight = input_tensor(base, &[4])?;
	let velocity = input_tensor(base + 1, &[4])?;
	let gradient = input_tensor(base + 2, &[4])?;
	let updated_velocity = output_tensor(base + 3, &[4])?;
	let updated_weight = output_tensor(base + 4, &[4])?;
	let inputs = [
		NamedTensor::new("parameters", &weight),
		NamedTensor::new("velocity", &velocity),
		NamedTensor::new("gradient", &gradient),
	];
	let outputs = [
		NamedTensor::new("updated_velocity", &updated_velocity),
		NamedTensor::new("updated_parameters", &updated_weight),
	];
	let prepared = parameters(&[
		("momentum", f32_parameter(0.9)),
		("learning_rate", f32_parameter(0.01)),
	]);
	Ok(materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_momentum_update")?,
		&inputs,
		&outputs,
		"parameters",
		&prepared,
		identities,
		ByteCount::new(0),
	))?)
}

#[test]
fn explicit_identity_ranges_assemble_repeated_operations_without_collisions() -> TestResult {
	let first_namespace = namespace(10_000);
	let second_namespace = namespace(20_000);
	validate_identity_namespaces(&[first_namespace, second_namespace])?;
	let first = momentum_fragment(1, first_namespace)?;
	let second = momentum_fragment(101, second_namespace)?;
	let graph = CalculationGraph::assemble(
		[first.graph().clone(), second.graph().clone()],
		[
			ValueId::new(1),
			ValueId::new(2),
			ValueId::new(3),
			ValueId::new(101),
			ValueId::new(102),
			ValueId::new(103),
		],
		[
			ValueId::new(4),
			ValueId::new(5),
			ValueId::new(104),
			ValueId::new(105),
		],
	)?;
	assert_eq!(graph.nodes.len(), 4);
	assert_eq!(
		graph.nodes
			.iter()
			.map(|node| node.kernel.id)
			.collect::<BTreeSet<_>>()
			.len(),
		4
	);
	assert_eq!(first.identity_namespace(), first_namespace);
	assert_eq!(second.identity_namespace(), second_namespace);

	let overlap = validate_identity_namespaces(&[
		namespace(30_000),
		IdentityNamespace::new(ValueId::new(30_999), 10, KernelTemplateId::new(40_000), 10),
	]);
	let error = match overlap {
		Err(error) => error,
		Ok(()) => return Err("overlapping identity ranges were accepted".into()),
	};
	assert_eq!(error.kind, OperationErrorKind::IdentityNamespaceOverlap);

	let exhausted = momentum_fragment(
		201,
		IdentityNamespace::new(ValueId::new(50_000), 1, KernelTemplateId::new(50_000), 1),
	);
	let error = match exhausted {
		Err(error) => error,
		Ok(_) => return Err("exhausted kernel identity range was accepted".into()),
	};
	let operation = error
		.downcast_ref::<crate::OperationError>()
		.ok_or("identity exhaustion returned a foreign error")?;
	assert_eq!(
		operation.kind,
		OperationErrorKind::IdentityNamespaceExhausted
	);
	Ok(())
}

#[test]
fn materializes_adaptive_optimizer_abis_and_exact_workspace() -> TestResult {
	for (index, symbol) in ["gpu_adagrad_update", "gpu_rmsprop_update"]
		.into_iter()
		.enumerate()
	{
		let gradient = input_tensor(1, &[4])?;
		let weight = input_tensor(2, &[4])?;
		let state = input_tensor(3, &[4])?;
		let updated_state = output_tensor(4, &[4])?;
		let updated_weight = output_tensor(5, &[4])?;
		let state_name = if symbol == "gpu_adagrad_update" {
			"accumulator"
		} else {
			"cache"
		};
		let updated_state_name = if symbol == "gpu_adagrad_update" {
			"updated_accumulator"
		} else {
			"updated_cache"
		};
		let inputs = [
			NamedTensor::new("gradient", &gradient),
			NamedTensor::new("weight", &weight),
			NamedTensor::new(state_name, &state),
		];
		let outputs = [
			NamedTensor::new(updated_state_name, &updated_state),
			NamedTensor::new("updated_weight", &updated_weight),
		];
		let prepared = match symbol {
			"gpu_adagrad_update" => parameters(&[
				("learning_rate", f32_parameter(0.01)),
				("epsilon", f32_parameter(1.0e-6)),
			]),
			"gpu_rmsprop_update" => parameters(&[
				("learning_rate", f32_parameter(0.01)),
				("epsilon", f32_parameter(1.0e-6)),
				("decay", f32_parameter(0.9)),
			]),
			_ => return Err("unexpected adaptive optimizer symbol".into()),
		};
		let materialized = materialize_composition(MaterializationRequest::new(
			operation_registry().resolve_unique(symbol)?,
			&inputs,
			&outputs,
			"weight",
			&prepared,
			namespace(40_000 + u64::try_from(index)? * 2_000),
			ByteCount::new(0),
		))?;
		assert_graph(&materialized, 0)?;
	}
	Ok(())
}

#[test]
fn materializes_adam_nadam_and_lion_with_owned_state_dependencies() -> TestResult {
	for (index, symbol) in ["gpu_adam_update", "gpu_adamw_update", "gpu_nadam_update"]
		.into_iter()
		.enumerate()
	{
		let gradient = input_tensor(1, &[4])?;
		let weight = input_tensor(2, &[4])?;
		let first = input_tensor(3, &[4])?;
		let second = input_tensor(4, &[4])?;
		let updated_first = output_tensor(5, &[4])?;
		let updated_second = output_tensor(6, &[4])?;
		let updated_weight = output_tensor(7, &[4])?;
		let inputs = [
			NamedTensor::new("gradient", &gradient),
			NamedTensor::new("weight", &weight),
			NamedTensor::new("first_moment", &first),
			NamedTensor::new("second_moment", &second),
		];
		let outputs = [
			NamedTensor::new("updated_first_moment", &updated_first),
			NamedTensor::new("updated_second_moment", &updated_second),
			NamedTensor::new("updated_weight", &updated_weight),
		];
		let mut prepared = parameters(&[
			("learning_rate", f32_parameter(0.01)),
			("beta_one", f32_parameter(0.9)),
			("beta_two", f32_parameter(0.999)),
			("epsilon", f32_parameter(1.0e-6)),
			("step", PreparedParameter::U64(5)),
		]);
		if symbol == "gpu_adamw_update" {
			prepared.insert("weight_decay".to_owned(), f32_parameter(0.01));
		}
		let materialized = materialize_composition(MaterializationRequest::new(
			operation_registry().resolve_unique(symbol)?,
			&inputs,
			&outputs,
			"weight",
			&prepared,
			namespace(50_000 + u64::try_from(index)? * 2_000),
			ByteCount::new(0),
		))?;
		assert_graph(&materialized, 0)?;
		assert_eq!(
			materialized
				.graph()
				.dependencies(materialized.graph().nodes[1].kernel.id)?,
			[materialized.graph().nodes[0].kernel.id]
		);
	}

	let gradient = input_tensor(11, &[4])?;
	let weight = input_tensor(12, &[4])?;
	let moment = input_tensor(13, &[4])?;
	let updated_moment = output_tensor(14, &[4])?;
	let updated_weight = output_tensor(15, &[4])?;
	let inputs = [
		NamedTensor::new("gradient", &gradient),
		NamedTensor::new("weight", &weight),
		NamedTensor::new("moment", &moment),
	];
	let outputs = [
		NamedTensor::new("updated_moment", &updated_moment),
		NamedTensor::new("updated_weight", &updated_weight),
	];
	let prepared = parameters(&[
		("learning_rate", f32_parameter(0.01)),
		("beta_one", f32_parameter(0.9)),
		("beta_two", f32_parameter(0.99)),
		("weight_decay", f32_parameter(0.01)),
	]);
	let lion = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_lion_update")?,
		&inputs,
		&outputs,
		"weight",
		&prepared,
		namespace(60_000),
		ByteCount::new(16),
	))?;
	assert_graph(&lion, 16)
}

#[test]
fn materializes_lamb_phases_and_gradient_clip() -> TestResult {
	let gradient = input_tensor(1, &[4])?;
	let weight = input_tensor(2, &[4])?;
	let moment = input_tensor(3, &[4])?;
	let velocity = input_tensor(4, &[4])?;
	let updated_moment = output_tensor(5, &[4])?;
	let updated_velocity = output_tensor(6, &[4])?;
	let update = output_tensor(7, &[4])?;
	let weight_norm_squared = output_tensor(8, &[1])?;
	let update_norm_squared = output_tensor(9, &[1])?;
	let inputs = [
		NamedTensor::new("gradient", &gradient),
		NamedTensor::new("weight", &weight),
		NamedTensor::new("moment", &moment),
		NamedTensor::new("velocity", &velocity),
	];
	let outputs = [
		NamedTensor::new("updated_moment", &updated_moment),
		NamedTensor::new("updated_velocity", &updated_velocity),
		NamedTensor::new("update", &update),
		NamedTensor::new("weight_norm_squared", &weight_norm_squared),
		NamedTensor::new("update_norm_squared", &update_norm_squared),
	];
	let prepared = parameters(&[
		("beta_one", f32_parameter(0.9)),
		("beta_two", f32_parameter(0.999)),
		("epsilon", f32_parameter(1.0e-6)),
		("weight_decay", f32_parameter(0.01)),
		("step", PreparedParameter::U64(5)),
		("tree_lanes", PreparedParameter::U64(64)),
	]);
	let phase_one = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_lamb_phase1")?,
		&inputs,
		&outputs,
		"weight",
		&prepared,
		namespace(70_000),
		ByteCount::new(48),
	))?;
	assert_graph(&phase_one, 48)?;

	let update = input_tensor(21, &[4])?;
	let weight_norm_squared = input_tensor(22, &[1])?;
	let update_norm_squared = input_tensor(23, &[1])?;
	let weight = input_tensor(24, &[4])?;
	let updated_weight = output_tensor(25, &[4])?;
	let inputs = [
		NamedTensor::new("update", &update),
		NamedTensor::new("weight_norm_squared", &weight_norm_squared),
		NamedTensor::new("update_norm_squared", &update_norm_squared),
		NamedTensor::new("weight", &weight),
	];
	let outputs = [NamedTensor::new("updated_weight", &updated_weight)];
	let prepared = parameters(&[
		("learning_rate", f32_parameter(0.01)),
		("tree_lanes", PreparedParameter::U64(64)),
	]);
	let phase_two = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_lamb_phase2")?,
		&inputs,
		&outputs,
		"weight",
		&prepared,
		namespace(72_000),
		ByteCount::new(16),
	))?;
	assert_graph(&phase_two, 16)?;

	let values = input_tensor(31, &[4])?;
	let clipped = output_tensor(32, &[4])?;
	let inputs = [NamedTensor::new("values", &values)];
	let outputs = [NamedTensor::new("clipped", &clipped)];
	let prepared = parameters(&[
		("maximum_norm", f32_parameter(1.0)),
		("tree_lanes", PreparedParameter::U64(64)),
	]);
	let clipped = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_grad_clip_norm")?,
		&inputs,
		&outputs,
		"values",
		&prepared,
		namespace(74_000),
		ByteCount::new(20),
	))?;
	assert_graph(&clipped, 20)
}

#[test]
fn materializes_batch_normalization_training_backward_and_running_update() -> TestResult {
	let values = input_tensor(1, &[2, 3])?;
	let scale = input_tensor(2, &[3])?;
	let bias = input_tensor(3, &[3])?;
	let normalized = output_tensor(4, &[2, 3])?;
	let saved_mean = output_tensor(5, &[3])?;
	let saved_inverse = output_tensor(6, &[3])?;
	let inputs = [
		NamedTensor::new("values", &values),
		NamedTensor::new("scale", &scale),
		NamedTensor::new("bias", &bias),
	];
	let outputs = [
		NamedTensor::new("normalized", &normalized),
		NamedTensor::new("saved_mean", &saved_mean),
		NamedTensor::new("saved_inverse_standard_deviation", &saved_inverse),
	];
	let prepared = parameters(&[
		("epsilon", f32_parameter(1.0e-5)),
		("tree_lanes", PreparedParameter::U64(64)),
	]);
	let forward = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_batchnorm_forward")?,
		&inputs,
		&outputs,
		"values",
		&prepared,
		namespace(80_000),
		ByteCount::new(72),
	))?;
	assert_graph(&forward, 72)?;

	let output_gradient = input_tensor(11, &[2, 3])?;
	let values = input_tensor(12, &[2, 3])?;
	let saved_mean = input_tensor(13, &[3])?;
	let saved_inverse = input_tensor(14, &[3])?;
	let scale = input_tensor(15, &[3])?;
	let input_gradient = output_tensor(16, &[2, 3])?;
	let scale_gradient = output_tensor(17, &[3])?;
	let bias_gradient = output_tensor(18, &[3])?;
	let inputs = [
		NamedTensor::new("output_gradient", &output_gradient),
		NamedTensor::new("values", &values),
		NamedTensor::new("saved_mean", &saved_mean),
		NamedTensor::new("saved_inverse_standard_deviation", &saved_inverse),
		NamedTensor::new("scale", &scale),
	];
	let outputs = [
		NamedTensor::new("input_gradient", &input_gradient),
		NamedTensor::new("scale_gradient", &scale_gradient),
		NamedTensor::new("bias_gradient", &bias_gradient),
	];
	let prepared = parameters(&[("tree_lanes", PreparedParameter::U64(64))]);
	let backward = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_batchnorm_backward")?,
		&inputs,
		&outputs,
		"values",
		&prepared,
		namespace(82_000),
		ByteCount::new(120),
	))?;
	assert_graph(&backward, 120)?;

	let saved_mean = input_tensor(21, &[3])?;
	let saved_variance = input_tensor(22, &[3])?;
	let running_mean = input_tensor(23, &[3])?;
	let running_variance = input_tensor(24, &[3])?;
	let updated_running_mean = output_tensor(25, &[3])?;
	let updated_running_variance = output_tensor(26, &[3])?;
	let inputs = [
		NamedTensor::new("saved_mean", &saved_mean),
		NamedTensor::new("saved_variance", &saved_variance),
		NamedTensor::new("running_mean", &running_mean),
		NamedTensor::new("running_variance", &running_variance),
	];
	let outputs = [
		NamedTensor::new("updated_running_mean", &updated_running_mean),
		NamedTensor::new("updated_running_variance", &updated_running_variance),
	];
	let prepared = parameters(&[("momentum", f32_parameter(0.1))]);
	let update = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_bn_update_running")?,
		&inputs,
		&outputs,
		"running_mean",
		&prepared,
		namespace(84_000),
		ByteCount::new(0),
	))?;
	assert_graph(&update, 0)
}

fn materialize_layer_forward(symbol: &str, identities: IdentityNamespace) -> TestResult<MaterializedComposition> {
	let values = input_tensor(1, &[2, 3])?;
	let scale = input_tensor(2, &[3])?;
	let bias = input_tensor(3, &[3])?;
	let normalized = output_tensor(4, &[2, 3])?;
	let inputs = [
		NamedTensor::new("values", &values),
		NamedTensor::new("scale", &scale),
		NamedTensor::new("bias", &bias),
	];
	let outputs = [NamedTensor::new("normalized", &normalized)];
	let mut prepared = parameters(&[
		("epsilon", f32_parameter(1.0e-5)),
		("tree_lanes", PreparedParameter::U64(64)),
	]);
	if symbol == "gpu_layernorm_opt_into" {
		prepared.insert("has_scale".to_owned(), PreparedParameter::Bool(true));
		prepared.insert("has_bias".to_owned(), PreparedParameter::Bool(true));
	}
	Ok(materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique(symbol)?,
		&inputs,
		&outputs,
		"values",
		&prepared,
		identities,
		ByteCount::new(64),
	))?)
}

fn materialize_layer_backward(symbol: &str, identities: IdentityNamespace) -> TestResult<MaterializedComposition> {
	let output_gradient = input_tensor(1, &[2, 3])?;
	let values = input_tensor(2, &[2, 3])?;
	let scale = input_tensor(3, &[3])?;
	let input_gradient = output_tensor(4, &[2, 3])?;
	let scale_gradient = output_tensor(5, &[3])?;
	let bias_gradient = output_tensor(6, &[3])?;
	let inputs = [
		NamedTensor::new("output_gradient", &output_gradient),
		NamedTensor::new("values", &values),
		NamedTensor::new("scale", &scale),
	];
	let outputs = [
		NamedTensor::new("input_gradient", &input_gradient),
		NamedTensor::new("scale_gradient", &scale_gradient),
		NamedTensor::new("bias_gradient", &bias_gradient),
	];
	let prepared = parameters(&[
		("epsilon", f32_parameter(1.0e-5)),
		("tree_lanes", PreparedParameter::U64(64)),
	]);
	Ok(materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique(symbol)?,
		&inputs,
		&outputs,
		"values",
		&prepared,
		identities,
		ByteCount::new(176),
	))?)
}

#[test]
fn every_source_qualified_layer_normalization_alias_materializes() -> TestResult {
	for (index, symbol) in [
		"gpu_layernorm_f32",
		"gpu_layernorm_into",
		"gpu_layernorm_opt_into",
	]
	.into_iter()
	.enumerate()
	{
		let materialized = materialize_layer_forward(symbol, namespace(90_000 + u64::try_from(index)? * 2_000))?;
		assert_graph(&materialized, 64)?;
	}
	for (index, symbol) in [
		"gpu_layernorm_backward_f32",
		"gpu_layernorm_backward_full_into",
	]
	.into_iter()
	.enumerate()
	{
		let materialized = materialize_layer_backward(symbol, namespace(96_000 + u64::try_from(index)? * 2_000))?;
		assert_graph(&materialized, 176)?;
	}
	Ok(())
}

fn materialize_rms_forward(
	symbol: &str,
	has_scale: bool,
	identities: IdentityNamespace,
) -> TestResult<MaterializedComposition> {
	let values = input_tensor(1, &[2, 3])?;
	let scale = input_tensor(2, &[3])?;
	let normalized = output_tensor(3, &[2, 3])?;
	let mut inputs = vec![NamedTensor::new("values", &values)];
	if has_scale {
		inputs.push(NamedTensor::new("scale", &scale));
	}
	let outputs = [NamedTensor::new("normalized", &normalized)];
	let prepared = parameters(&[
		("epsilon", f32_parameter(1.0e-5)),
		("tree_lanes", PreparedParameter::U64(64)),
	]);
	Ok(materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique(symbol)?,
		&inputs,
		&outputs,
		"values",
		&prepared,
		identities,
		ByteCount::new(40),
	))?)
}

#[test]
fn every_canonical_rms_normalization_alias_and_backward_materializes() -> TestResult {
	for (index, (symbol, has_scale)) in [
		("gpu_rmsnorm", true),
		("gpu_rmsnorm_f64", true),
		("gpu_rmsnorm_f64_nogamma", false),
	]
	.into_iter()
	.enumerate()
	{
		let materialized = materialize_rms_forward(
			symbol,
			has_scale,
			namespace(100_000 + u64::try_from(index)? * 2_000),
		)?;
		assert_graph(&materialized, 40)?;
	}

	let output_gradient = input_tensor(11, &[2, 3])?;
	let values = input_tensor(12, &[2, 3])?;
	let scale = input_tensor(13, &[3])?;
	let input_gradient = output_tensor(14, &[2, 3])?;
	let scale_gradient = output_tensor(15, &[3])?;
	let inputs = [
		NamedTensor::new("output_gradient", &output_gradient),
		NamedTensor::new("values", &values),
		NamedTensor::new("scale", &scale),
	];
	let outputs = [
		NamedTensor::new("input_gradient", &input_gradient),
		NamedTensor::new("scale_gradient", &scale_gradient),
	];
	let prepared = parameters(&[
		("epsilon", f32_parameter(1.0e-5)),
		("tree_lanes", PreparedParameter::U64(64)),
	]);
	let backward = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_rmsnorm_backward")?,
		&inputs,
		&outputs,
		"values",
		&prepared,
		namespace(106_000),
		ByteCount::new(88),
	))?;
	assert_graph(&backward, 88)
}

#[test]
fn source_qualified_concrete_manifest_count_is_exact() {
	let remaining = crate::remaining_composition_manifest();
	let structured = operation_registry()
		.iter()
		.filter(|descriptor| {
			matches!(
				descriptor.lowering,
				crate::LoweringAvailability::Composition(_)
			)
	})
		.count();
	assert_eq!(structured, 264);
	assert_eq!(remaining.len(), 138);
	assert_eq!(structured - remaining.len(), 126);

	let mapped = operation_registry()
		.iter()
		.filter(|descriptor| {
			remaining
				.iter()
				.all(|entry| entry.operation() != descriptor.id)
				&& matches!(
					descriptor.lowering,
					crate::LoweringAvailability::Composition(_)
				)
		})
		.map(|descriptor| (descriptor.symbol, descriptor.source))
		.collect::<BTreeSet<_>>();
	assert_eq!(mapped.len(), 126);
}

#[test]
fn new_materializers_reject_non_exact_abis_and_invalid_prepared_values() -> TestResult {
	let gradient = input_tensor(1, &[4])?;
	let weight = input_tensor(2, &[4])?;
	let accumulator = input_tensor(3, &[4])?;
	let unexpected = input_tensor(4, &[4])?;
	let updated_accumulator = output_tensor(5, &[4])?;
	let updated_weight = output_tensor(6, &[4])?;
	let inputs = [
		NamedTensor::new("gradient", &gradient),
		NamedTensor::new("weight", &weight),
		NamedTensor::new("accumulator", &accumulator),
		NamedTensor::new("unexpected", &unexpected),
	];
	let outputs = [
		NamedTensor::new("updated_accumulator", &updated_accumulator),
		NamedTensor::new("updated_weight", &updated_weight),
	];
	let prepared = parameters(&[
		("learning_rate", f32_parameter(0.01)),
		("epsilon", f32_parameter(1.0e-6)),
	]);
	let result = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_adagrad_update")?,
		&inputs,
		&outputs,
		"weight",
		&prepared,
		namespace(110_000),
		ByteCount::new(0),
	));
	let error = match result {
		Err(error) => error,
		Ok(_) => return Err("extra optimizer tensor was accepted".into()),
	};
	assert_eq!(
		error.kind,
		OperationErrorKind::InvalidMaterializationRequest
	);

	let gradient = input_tensor(11, &[4])?;
	let weight = input_tensor(12, &[4])?;
	let first = input_tensor(13, &[4])?;
	let second = input_tensor(14, &[4])?;
	let updated_first = output_tensor(15, &[4])?;
	let updated_second = output_tensor(16, &[4])?;
	let updated_weight = output_tensor(17, &[4])?;
	let inputs = [
		NamedTensor::new("gradient", &gradient),
		NamedTensor::new("weight", &weight),
		NamedTensor::new("first_moment", &first),
		NamedTensor::new("second_moment", &second),
	];
	let outputs = [
		NamedTensor::new("updated_first_moment", &updated_first),
		NamedTensor::new("updated_second_moment", &updated_second),
		NamedTensor::new("updated_weight", &updated_weight),
	];
	let prepared = parameters(&[
		("learning_rate", f32_parameter(0.01)),
		("beta_one", f32_parameter(0.9)),
		("beta_two", f32_parameter(0.999)),
		("epsilon", f32_parameter(1.0e-6)),
		("step", PreparedParameter::U64(0)),
	]);
	let result = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_adam_update")?,
		&inputs,
		&outputs,
		"weight",
		&prepared,
		namespace(112_000),
		ByteCount::new(0),
	));
	let error = match result {
		Err(error) => error,
		Ok(_) => return Err("zero Adam step was accepted".into()),
	};
	assert_eq!(
		error.kind,
		OperationErrorKind::InvalidMaterializationRequest
	);
	Ok(())
}
