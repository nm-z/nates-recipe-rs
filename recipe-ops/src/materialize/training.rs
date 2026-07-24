use recipe_core::{DType, ScalarOpcode};
use recipe_language::{AxisSet, Contraction, Elementwise, PrimitiveKind, Reduce, ReduceOperator, ReduceResult, Tensor};

use crate::{OperationDescriptor, OperationId, OperationResult};

use super::{
	Emitter, FamilyDispatch, MaterializationRequest, input as requested_input, language_error, output,
	prepared_tree_lanes, request_error, require_dtype, require_exact_abi, require_shape, scalar_binary,
	scalar_builder, scalar_finish, scalar_input,
};

const OPERATIONS: &[(&str, &str)] = &[
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

pub(super) fn supports(descriptor: OperationDescriptor) -> bool {
	OPERATIONS.contains(&(descriptor.symbol, descriptor.source))
}

pub(super) fn dispatch(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> FamilyDispatch {
	let result = match supports(request.descriptor) {
		false => return FamilyDispatch::NotOwned,
		true => match request.descriptor.symbol {
			"gpu_bce_with_logits" => emit_binary_cross_entropy_with_logits(request, emitter),
			"gpu_linear_backward_full_into" => emit_linear_backward_full(request, emitter),
			"gpu_linear_backward_weights_only_into" => emit_linear_backward_weights_only(request, emitter),
			"gpu_linear_f32" | "gpu_linear_into" => emit_linear(request, emitter),
			"gpu_matvec_bias_into" => emit_matvec_bias(request, emitter),
			symbol => Err(request_error(
				request,
				format!("training-operation dispatch is incomplete for {symbol}"),
			)),
		},
	};
	FamilyDispatch::Owned(result)
}

fn emit_linear(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(request, &["input", "weight", "bias"], &["output"], &[])?;
	require_iteration_input(request, "input")?;
	let input = requested_input(request, "input")?;
	let weight = requested_input(request, "weight")?;
	let bias = requested_input(request, "bias")?;
	let output = output(request, "output")?;
	let (rows, inner) = require_f32_matrix(request, input, "input")?;
	let (weight_inner, columns) = require_f32_matrix(request, weight, "weight")?;
	if weight_inner != inner {
		return Err(request_error(
			request,
			format!("weight leading extent {weight_inner} does not match input inner extent {inner}"),
		));
	}
	require_f32_shape(request, bias, &[columns], "bias")?;
	require_f32_shape(request, output, &[rows, columns], "output")?;

	let contracted = emitter.intermediate(DType::F32, output.shape.clone())?;
	emitter.emit(
		vec![input.id, weight.id],
		vec![contracted],
		PrimitiveKind::Contraction(Contraction {
			batch_axes: Vec::new(),
			contract_axes: vec![(1, 0)],
		}),
	)?;
	emitter.emit(
		vec![contracted, bias.id],
		vec![output.id],
		PrimitiveKind::Elementwise(Elementwise {
			program: add_program(request.descriptor.id)?,
		}),
	)
}

fn emit_matvec_bias(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(request, &["input", "weight", "bias"], &["output"], &[])?;
	require_iteration_input(request, "input")?;
	let input = requested_input(request, "input")?;
	let weight = requested_input(request, "weight")?;
	let bias = requested_input(request, "bias")?;
	let output = output(request, "output")?;
	let (rows, inner) = require_f32_matrix(request, input, "input")?;
	require_f32_shape(request, weight, &[inner], "weight")?;
	require_f32_shape(request, bias, &[1], "bias")?;
	require_f32_shape(request, output, &[rows], "output")?;

	let contracted = emitter.intermediate(DType::F32, output.shape.clone())?;
	emitter.emit(
		vec![input.id, weight.id],
		vec![contracted],
		PrimitiveKind::Contraction(Contraction {
			batch_axes: Vec::new(),
			contract_axes: vec![(1, 0)],
		}),
	)?;
	emitter.emit(
		vec![contracted, bias.id],
		vec![output.id],
		PrimitiveKind::Elementwise(Elementwise {
			program: add_program(request.descriptor.id)?,
		}),
	)
}

fn emit_linear_backward_full(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(
		request,
		&["output_gradient", "input", "weight"],
		&["input_gradient", "weight_gradient", "bias_gradient"],
		&["tree_lanes"],
	)?;
	require_iteration_input(request, "output_gradient")?;
	let output_gradient = requested_input(request, "output_gradient")?;
	let input = requested_input(request, "input")?;
	let weight = requested_input(request, "weight")?;
	let input_gradient = output(request, "input_gradient")?;
	let weight_gradient = output(request, "weight_gradient")?;
	let bias_gradient = output(request, "bias_gradient")?;
	let (rows, columns) = require_f32_matrix(request, output_gradient, "output_gradient")?;
	let (input_rows, inner) = require_f32_matrix(request, input, "input")?;
	if input_rows != rows {
		return Err(request_error(
			request,
			format!("input row extent {input_rows} does not match output-gradient row extent {rows}"),
		));
	}
	require_f32_shape(request, weight, &[inner, columns], "weight")?;
	require_f32_shape(request, input_gradient, &[rows, inner], "input_gradient")?;
	require_f32_shape(
		request,
		weight_gradient,
		&[inner, columns],
		"weight_gradient",
	)?;
	require_f32_shape(request, bias_gradient, &[columns], "bias_gradient")?;
	let tree_lanes = prepared_tree_lanes(request)?;

	emitter.emit(
		vec![output_gradient.id, weight.id],
		vec![input_gradient.id],
		PrimitiveKind::Contraction(Contraction {
			batch_axes: Vec::new(),
			contract_axes: vec![(1, 1)],
		}),
	)?;
	emit_weight_gradient(request, emitter, input, output_gradient, weight_gradient)?;
	emit_bias_gradient(request, emitter, output_gradient, bias_gradient, tree_lanes)
}

fn emit_linear_backward_weights_only(
	request: &MaterializationRequest<'_>,
	emitter: &mut Emitter<'_>,
) -> OperationResult<()> {
	require_exact_abi(
		request,
		&["output_gradient", "input"],
		&["weight_gradient", "bias_gradient"],
		&["tree_lanes"],
	)?;
	require_iteration_input(request, "output_gradient")?;
	let output_gradient = requested_input(request, "output_gradient")?;
	let input = requested_input(request, "input")?;
	let weight_gradient = output(request, "weight_gradient")?;
	let bias_gradient = output(request, "bias_gradient")?;
	let (rows, columns) = require_f32_matrix(request, output_gradient, "output_gradient")?;
	let (input_rows, inner) = require_f32_matrix(request, input, "input")?;
	if input_rows != rows {
		return Err(request_error(
			request,
			format!("input row extent {input_rows} does not match output-gradient row extent {rows}"),
		));
	}
	require_f32_shape(
		request,
		weight_gradient,
		&[inner, columns],
		"weight_gradient",
	)?;
	require_f32_shape(request, bias_gradient, &[columns], "bias_gradient")?;
	let tree_lanes = prepared_tree_lanes(request)?;

	emit_weight_gradient(request, emitter, input, output_gradient, weight_gradient)?;
	emit_bias_gradient(request, emitter, output_gradient, bias_gradient, tree_lanes)
}

fn emit_weight_gradient(
	_request: &MaterializationRequest<'_>,
	emitter: &mut Emitter<'_>,
	input: &Tensor,
	output_gradient: &Tensor,
	weight_gradient: &Tensor,
) -> OperationResult<()> {
	emitter.emit(
		vec![input.id, output_gradient.id],
		vec![weight_gradient.id],
		PrimitiveKind::Contraction(Contraction {
			batch_axes: Vec::new(),
			contract_axes: vec![(0, 0)],
		}),
	)
}

fn emit_bias_gradient(
	request: &MaterializationRequest<'_>,
	emitter: &mut Emitter<'_>,
	output_gradient: &Tensor,
	bias_gradient: &Tensor,
	tree_lanes: u32,
) -> OperationResult<()> {
	let axes = AxisSet::new(vec![0]).map_err(|error| language_error(request.descriptor.id, error.to_string()))?;
	emitter.emit(
		vec![output_gradient.id],
		vec![bias_gradient.id],
		PrimitiveKind::Reduce(Reduce {
			operator: ReduceOperator::Sum,
			axes,
			keep_dimensions: false,
			result: ReduceResult::Value,
			tree_lanes,
		}),
	)
}

fn emit_binary_cross_entropy_with_logits(
	request: &MaterializationRequest<'_>,
	emitter: &mut Emitter<'_>,
) -> OperationResult<()> {
	require_exact_abi(
		request,
		&["logits", "targets"],
		&["losses", "gradients"],
		&[],
	)?;
	require_iteration_input(request, "logits")?;
	let logits = requested_input(request, "logits")?;
	let targets = requested_input(request, "targets")?;
	let losses = output(request, "losses")?;
	let gradients = output(request, "gradients")?;
	require_dtype(request, logits, DType::F32, "logits")?;
	require_f32_shape(request, targets, logits.shape.extents(), "targets")?;
	require_f32_shape(request, losses, logits.shape.extents(), "losses")?;
	require_f32_shape(request, gradients, logits.shape.extents(), "gradients")?;
	let program = crate::scalar::binary_cross_entropy_with_logits_program()
		.map_err(|error| error.for_operation(request.descriptor.id))?;
	emitter.emit(
		vec![logits.id, targets.id],
		vec![losses.id, gradients.id],
		PrimitiveKind::Elementwise(Elementwise { program }),
	)
}

fn require_iteration_input(request: &MaterializationRequest<'_>, expected: &str) -> OperationResult<()> {
	if request.iteration_shape_input == expected {
		Ok(())
	} else {
		Err(request_error(
			request,
			format!(
				"iteration shape input must be {expected:?}, got {:?}",
				request.iteration_shape_input
			),
		))
	}
}

fn require_f32_matrix(
	request: &MaterializationRequest<'_>,
	tensor: &Tensor,
	name: &str,
) -> OperationResult<(u64, u64)> {
	require_dtype(request, tensor, DType::F32, name)?;
	match tensor.shape.extents() {
		[rows, columns] => Ok((*rows, *columns)),
		shape => Err(request_error(
			request,
			format!("{name} has shape {shape:?}, expected a rank-two f32 matrix"),
		)),
	}
}

fn require_f32_shape(
	request: &MaterializationRequest<'_>,
	tensor: &Tensor,
	shape: &[u64],
	name: &str,
) -> OperationResult<()> {
	require_dtype(request, tensor, DType::F32, name)?;
	require_shape(request, tensor, shape, name)
}

fn add_program(operation: OperationId) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let value = scalar_input(operation, &mut builder, DType::F32)?;
	let bias = scalar_input(operation, &mut builder, DType::F32)?;
	let output = scalar_binary(operation, &mut builder, ScalarOpcode::Add, value, bias)?;
	scalar_finish(operation, builder, &[output])
}
