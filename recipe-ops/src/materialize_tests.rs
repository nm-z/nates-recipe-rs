use std::collections::BTreeMap;
use std::error::Error;

use recipe_core::{ByteCount, DType, KernelTemplateId, ValueId};
use recipe_language::{PrimitiveKind, Shape, Tensor};

use crate::{
	IdentityNamespace, LoweringAvailability, MaterializationRequest, MissingConcreteComponent, NamedTensor,
	OperationErrorKind, PreparedParameter, PreparedParameters, PrimitiveFamily, expand_composition,
	materialize_composition, operation_registry, remaining_composition_manifest,
};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

fn input_tensor(id: u64, dtype: DType, extents: &[u64]) -> TestResult<Tensor> {
	Ok(Tensor::contiguous(
		ValueId::new(id),
		dtype,
		Shape::new(extents.to_vec())?,
		true,
		false,
	)?)
}

fn output_tensor(id: u64, dtype: DType, extents: &[u64]) -> TestResult<Tensor> {
	Ok(Tensor::contiguous(
		ValueId::new(id),
		dtype,
		Shape::new(extents.to_vec())?,
		false,
		true,
	)?)
}

fn parameters(values: &[(&str, PreparedParameter)]) -> PreparedParameters {
	values.iter()
		.map(|(name, value)| ((*name).to_owned(), *value))
		.collect()
}

const fn test_namespace() -> IdentityNamespace {
	IdentityNamespace::new(
		ValueId::new(10_000),
		10_000,
		KernelTemplateId::new(10_000),
		10_000,
	)
}

fn families(materialized: &crate::MaterializedComposition) -> Vec<PrimitiveFamily> {
	materialized
		.graph()
		.nodes
		.iter()
		.map(|node| match &node.kernel.kind {
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

fn resolved_families(materialized: &crate::MaterializedComposition) -> Vec<PrimitiveFamily> {
	materialized
		.resolved()
		.steps()
		.iter()
		.map(crate::ResolvedStep::family)
		.collect()
}

#[test]
fn resolves_shape_and_named_bounds_into_a_finite_dependency_dag() -> TestResult {
	let fft = operation_registry().resolve_unique("gpu_fft_c2c_1d")?;
	let shape = Shape::new(vec![9, 2])?;
	let expanded = expand_composition(fft, &shape, &BTreeMap::new())?;
	assert_eq!(expanded.bounds().len(), 1);
	assert_eq!(expanded.bounds()[0].value(), 4);
	assert_eq!(expanded.steps().len(), 12);
	for (ordinal, step) in expanded.steps().iter().enumerate() {
		assert_eq!(step.ordinal(), ordinal);
		assert_eq!(step.dependencies(), ordinal.checked_sub(1).as_slice());
		assert_eq!(step.iterations().len(), 1);
		assert_eq!(step.iterations()[0].index(), u64::try_from(ordinal / 3)?);
	}

	let svd = operation_registry().resolve_unique("gpu_svd")?;
	let prepared = parameters(&[("bidiagonal_qr_sweeps", PreparedParameter::U64(2))]);
	let expanded = expand_composition(svd, &Shape::new(vec![3, 5])?, &prepared)?;
	assert_eq!(
		expanded
			.bounds()
			.iter()
			.map(crate::ResolvedBound::value)
			.collect::<Vec<_>>(),
		[3, 2]
	);
	assert_eq!(expanded.steps().len(), 15);
	Ok(())
}

#[test]
fn materializes_an_exact_length_two_complex_fft_butterfly() -> TestResult {
	let specs = [
		("real", DType::F32),
		("imaginary", DType::F32),
		("stage_0_left_indices", DType::I32),
		("stage_0_right_indices", DType::I32),
		("stage_0_real_from_left_real", DType::F32),
		("stage_0_real_from_left_imaginary", DType::F32),
		("stage_0_real_from_right_real", DType::F32),
		("stage_0_real_from_right_imaginary", DType::F32),
		("stage_0_imaginary_from_left_real", DType::F32),
		("stage_0_imaginary_from_left_imaginary", DType::F32),
		("stage_0_imaginary_from_right_real", DType::F32),
		("stage_0_imaginary_from_right_imaginary", DType::F32),
		("stage_0_real_base", DType::F32),
		("stage_0_imaginary_base", DType::F32),
		("stage_0_destination_indices", DType::I32),
	];
	let tensors = specs
		.iter()
		.enumerate()
		.map(|(index, (_, dtype))| input_tensor(u64::try_from(index + 1)?, *dtype, &[2]))
		.collect::<TestResult<Vec<_>>>()?;
	let inputs = specs
		.iter()
		.zip(&tensors)
		.map(|((name, _), tensor)| NamedTensor::new(name, tensor))
		.collect::<Vec<_>>();
	let transformed_real = output_tensor(20, DType::F32, &[2])?;
	let transformed_imaginary = output_tensor(21, DType::F32, &[2])?;
	let outputs = [
		NamedTensor::new("transformed_real", &transformed_real),
		NamedTensor::new("transformed_imaginary", &transformed_imaginary),
	];
	let prepared = parameters(&[
		("fft_stage_tables_verified", PreparedParameter::Bool(true)),
		(
			"fft_destination_indices_unique",
			PreparedParameter::Bool(true),
		),
	]);
	let request = MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_fft_c2c_1d")?,
		&inputs,
		&outputs,
		"real",
		&prepared,
		test_namespace(),
		ByteCount::new(48),
	);
	let materialized = materialize_composition(request)?;
	assert_eq!(
		resolved_families(&materialized),
		[
			PrimitiveFamily::Gather,
			PrimitiveFamily::Elementwise,
			PrimitiveFamily::Scatter
		]
	);
	assert_eq!(
		families(&materialized),
		[
			PrimitiveFamily::Gather,
			PrimitiveFamily::Gather,
			PrimitiveFamily::Gather,
			PrimitiveFamily::Gather,
			PrimitiveFamily::Elementwise,
			PrimitiveFamily::Scatter,
			PrimitiveFamily::Scatter,
		]
	);
	assert_eq!(materialized.workspace().bytes().get(), 48);
	assert_eq!(materialized.stages().len(), 3);
	materialized.graph().validate()?;
	Ok(())
}

#[test]
fn materializes_every_static_solver_row_without_a_host_loop() -> TestResult {
	let specs = [
		("solution_base", DType::F32, 3_u64),
		("row_0_strict_prefix", DType::F32, 3),
		("row_0_rhs", DType::F32, 1),
		("row_0_diagonal", DType::F32, 1),
		("row_0_destination", DType::I32, 1),
		("row_1_strict_prefix", DType::F32, 3),
		("row_1_rhs", DType::F32, 1),
		("row_1_diagonal", DType::F32, 1),
		("row_1_destination", DType::I32, 1),
		("row_2_strict_prefix", DType::F32, 3),
		("row_2_rhs", DType::F32, 1),
		("row_2_diagonal", DType::F32, 1),
		("row_2_destination", DType::I32, 1),
	];
	let tensors = specs
		.iter()
		.enumerate()
		.map(|(index, (_, dtype, extent))| input_tensor(u64::try_from(index + 1)?, *dtype, &[*extent]))
		.collect::<TestResult<Vec<_>>>()?;
	let inputs = specs
		.iter()
		.zip(&tensors)
		.map(|((name, _, _), tensor)| NamedTensor::new(name, tensor))
		.collect::<Vec<_>>();
	let solution = output_tensor(20, DType::F32, &[3])?;
	let outputs = [NamedTensor::new("solution", &solution)];
	let prepared = parameters(&[
		("solution_base_zero", PreparedParameter::Bool(true)),
		("triangular_rows_verified", PreparedParameter::Bool(true)),
		("destination_indices_unique", PreparedParameter::Bool(true)),
		("tree_lanes", PreparedParameter::U64(64)),
	]);
	let materialized = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_tri_solve")?,
		&inputs,
		&outputs,
		"solution_base",
		&prepared,
		test_namespace(),
		ByteCount::new(84),
	))?;
	assert_eq!(
		resolved_families(&materialized),
		[
			PrimitiveFamily::Reduce,
			PrimitiveFamily::Elementwise,
			PrimitiveFamily::Scatter,
			PrimitiveFamily::Reduce,
			PrimitiveFamily::Elementwise,
			PrimitiveFamily::Scatter,
			PrimitiveFamily::Reduce,
			PrimitiveFamily::Elementwise,
			PrimitiveFamily::Scatter,
		]
	);
	assert_eq!(materialized.resolved().bounds()[0].value(), 3);
	assert_eq!(materialized.workspace().bytes().get(), 84);
	assert_eq!(materialized.graph().nodes.len(), 12);
	Ok(())
}

#[test]
fn materializes_attention_and_convolution_as_checked_prepared_gathers() -> TestResult {
	let packed = input_tensor(1, DType::F32, &[12])?;
	let indices = input_tensor(2, DType::I32, &[2, 3])?;
	let heads = output_tensor(3, DType::F32, &[2, 3])?;
	let inputs = [
		NamedTensor::new("packed", &packed),
		NamedTensor::new("indices", &indices),
	];
	let outputs = [NamedTensor::new("heads", &heads)];
	let prepared = parameters(&[("axis", PreparedParameter::U64(0))]);
	let attention = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_mha_split")?,
		&inputs,
		&outputs,
		"packed",
		&prepared,
		test_namespace(),
		ByteCount::new(24),
	))?;
	assert_eq!(
		families(&attention),
		[PrimitiveFamily::Gather, PrimitiveFamily::Elementwise]
	);
	assert_eq!(attention.workspace().bytes().get(), 24);

	let image = input_tensor(11, DType::F32, &[16])?;
	let receptive = input_tensor(12, DType::I32, &[3, 4])?;
	let columns = output_tensor(13, DType::F32, &[3, 4])?;
	let inputs = [
		NamedTensor::new("image", &image),
		NamedTensor::new("receptive_field_indices", &receptive),
	];
	let outputs = [NamedTensor::new("columns", &columns)];
	let prepared = parameters(&[
		("axis", PreparedParameter::U64(0)),
		(
			"indices_encode_receptive_fields",
			PreparedParameter::Bool(true),
		),
	]);
	let convolution = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_im2col_1d")?,
		&inputs,
		&outputs,
		"image",
		&prepared,
		test_namespace(),
		ByteCount::new(48),
	))?;
	assert_eq!(
		families(&convolution),
		[PrimitiveFamily::Gather, PrimitiveFamily::Elementwise]
	);
	assert_eq!(convolution.workspace().bytes().get(), 48);
	Ok(())
}

#[test]
fn materializes_normalization_and_optimizer_scalar_ssa() -> TestResult {
	let values = input_tensor(1, DType::F32, &[2, 3])?;
	let mean = input_tensor(2, DType::F32, &[3])?;
	let variance = input_tensor(3, DType::F32, &[3])?;
	let scale = input_tensor(4, DType::F32, &[3])?;
	let bias = input_tensor(5, DType::F32, &[3])?;
	let normalized = output_tensor(6, DType::F32, &[2, 3])?;
	let inputs = [
		NamedTensor::new("values", &values),
		NamedTensor::new("running_mean", &mean),
		NamedTensor::new("running_variance", &variance),
		NamedTensor::new("scale", &scale),
		NamedTensor::new("bias", &bias),
	];
	let outputs = [NamedTensor::new("normalized", &normalized)];
	let prepared = parameters(&[("epsilon", PreparedParameter::F32Bits(1.0e-5_f32.to_bits()))]);
	let normalization = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_batchnorm_inference")?,
		&inputs,
		&outputs,
		"values",
		&prepared,
		test_namespace(),
		ByteCount::new(0),
	))?;
	assert_eq!(families(&normalization), [PrimitiveFamily::Elementwise]);
	assert_eq!(normalization.workspace().bytes().get(), 0);

	let parameters_tensor = input_tensor(11, DType::F32, &[8])?;
	let velocity = input_tensor(12, DType::F32, &[8])?;
	let gradient = input_tensor(13, DType::F32, &[8])?;
	let updated_velocity = output_tensor(14, DType::F32, &[8])?;
	let updated_parameters = output_tensor(15, DType::F32, &[8])?;
	let inputs = [
		NamedTensor::new("parameters", &parameters_tensor),
		NamedTensor::new("velocity", &velocity),
		NamedTensor::new("gradient", &gradient),
	];
	let outputs = [
		NamedTensor::new("updated_velocity", &updated_velocity),
		NamedTensor::new("updated_parameters", &updated_parameters),
	];
	let prepared = parameters(&[
		("momentum", PreparedParameter::F32Bits(0.9_f32.to_bits())),
		(
			"learning_rate",
			PreparedParameter::F32Bits(0.01_f32.to_bits()),
		),
	]);
	let optimizer = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_momentum_update")?,
		&inputs,
		&outputs,
		"parameters",
		&prepared,
		test_namespace(),
		ByteCount::new(0),
	))?;
	assert_eq!(
		families(&optimizer),
		[PrimitiveFamily::Elementwise, PrimitiveFamily::Elementwise]
	);
	assert_eq!(
		optimizer
			.graph()
			.dependencies(optimizer.graph().nodes[1].kernel.id)?,
		[optimizer.graph().nodes[0].kernel.id]
	);
	Ok(())
}

#[test]
fn materializes_a_checked_tree_ml_routing_step() -> TestResult {
	let features = input_tensor(1, DType::F32, &[12])?;
	let feature_indices = input_tensor(2, DType::I32, &[4])?;
	let thresholds = input_tensor(3, DType::F32, &[4])?;
	let assignments = input_tensor(4, DType::I32, &[4])?;
	let destination_indices = input_tensor(5, DType::I32, &[4])?;
	let updated = output_tensor(6, DType::I32, &[4])?;
	let inputs = [
		NamedTensor::new("features", &features),
		NamedTensor::new("feature_indices", &feature_indices),
		NamedTensor::new("thresholds", &thresholds),
		NamedTensor::new("assignments", &assignments),
		NamedTensor::new("destination_indices", &destination_indices),
	];
	let outputs = [NamedTensor::new("updated_assignments", &updated)];
	let prepared = parameters(&[("destination_indices_unique", PreparedParameter::Bool(true))]);
	let materialized = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_leaf_split_apply")?,
		&inputs,
		&outputs,
		"features",
		&prepared,
		test_namespace(),
		ByteCount::new(32),
	))?;
	assert_eq!(
		families(&materialized),
		[
			PrimitiveFamily::Gather,
			PrimitiveFamily::Elementwise,
			PrimitiveFamily::Scatter
		]
	);
	assert_eq!(materialized.workspace().bytes().get(), 32);
	Ok(())
}

#[test]
fn workspace_and_missing_semantics_fail_closed_with_an_exact_manifest() -> TestResult {
	let specs = [
		("real", DType::F32),
		("imaginary", DType::F32),
		("stage_0_left_indices", DType::I32),
		("stage_0_right_indices", DType::I32),
		("stage_0_real_from_left_real", DType::F32),
		("stage_0_real_from_left_imaginary", DType::F32),
		("stage_0_real_from_right_real", DType::F32),
		("stage_0_real_from_right_imaginary", DType::F32),
		("stage_0_imaginary_from_left_real", DType::F32),
		("stage_0_imaginary_from_left_imaginary", DType::F32),
		("stage_0_imaginary_from_right_real", DType::F32),
		("stage_0_imaginary_from_right_imaginary", DType::F32),
		("stage_0_real_base", DType::F32),
		("stage_0_imaginary_base", DType::F32),
		("stage_0_destination_indices", DType::I32),
	];
	let tensors = specs
		.iter()
		.enumerate()
		.map(|(index, (_, dtype))| input_tensor(u64::try_from(index + 1)?, *dtype, &[2]))
		.collect::<TestResult<Vec<_>>>()?;
	let inputs = specs
		.iter()
		.zip(&tensors)
		.map(|((name, _), tensor)| NamedTensor::new(name, tensor))
		.collect::<Vec<_>>();
	let transformed_real = output_tensor(20, DType::F32, &[2])?;
	let transformed_imaginary = output_tensor(21, DType::F32, &[2])?;
	let outputs = [
		NamedTensor::new("transformed_real", &transformed_real),
		NamedTensor::new("transformed_imaginary", &transformed_imaginary),
	];
	let prepared = parameters(&[
		("fft_stage_tables_verified", PreparedParameter::Bool(true)),
		(
			"fft_destination_indices_unique",
			PreparedParameter::Bool(true),
		),
	]);
	let result = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_fft_c2c_1d")?,
		&inputs,
		&outputs,
		"real",
		&prepared,
		test_namespace(),
		ByteCount::new(47),
	));
	let error = match result {
		Err(error) => error,
		Ok(_) => {
			return Err(Box::<dyn Error>::from(
				"workspace-limited FFT unexpectedly materialized",
			));
		}
	};
	assert_eq!(error.kind, OperationErrorKind::WorkspaceLimitExceeded);

	let generic_input = input_tensor(20, DType::F32, &[4])?;
	let generic_output = output_tensor(21, DType::F32, &[4])?;
	let inputs = [NamedTensor::new("values", &generic_input)];
	let outputs = [NamedTensor::new("output", &generic_output)];
	let result = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_scaled_dot_product_attn")?,
		&inputs,
		&outputs,
		"values",
		&BTreeMap::new(),
		test_namespace(),
		ByteCount::new(0),
	));
	let error = match result {
		Err(error) => error,
		Ok(_) => {
			return Err(Box::<dyn Error>::from(
				"unmapped attention unexpectedly materialized",
			));
		}
	};
	assert_eq!(error.kind, OperationErrorKind::MissingConcreteFormula);

	let manifest = remaining_composition_manifest();
	let total_compositions = operation_registry()
		.iter()
		.filter(|descriptor| matches!(descriptor.lowering, LoweringAvailability::Composition(_)))
		.count();
	let concrete = total_compositions - manifest.len();
	assert_eq!(total_compositions, 266);
	assert_eq!(concrete, 134);
	assert_eq!(manifest.len(), 132);
	assert_eq!(manifest.len(), total_compositions - concrete);
	assert!(
		manifest
			.iter()
			.any(|entry| entry.symbol() == "gpu_scaled_dot_product_attn")
	);
	assert!(manifest.iter().all(|entry| entry.missing()
		== [
			MissingConcreteComponent::TensorAbi,
			MissingConcreteComponent::ScalarFormula,
			MissingConcreteComponent::PrimitiveParameters,
			MissingConcreteComponent::WorkspacePolicy,
		]));
	Ok(())
}
