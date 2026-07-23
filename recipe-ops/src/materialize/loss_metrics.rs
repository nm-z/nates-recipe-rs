use recipe_core::{DType, ScalarOpcode};
use recipe_language::{
	AxisSet, Elementwise, PrimitiveKind, Reduce, ReduceOperator, ReduceResult, ScalarProgramBuilder, Shape, Tensor,
};
use recipe_math::MathFunction;

use crate::{OperationDescriptor, OperationId, OperationResult};

use super::{
	Emitter, FamilyDispatch, KernelEmission, MaterializationRequest, exact_f32_extent, input, language_error, output,
	prepared_f32, prepared_tree_lanes, request_error, require_dtype, require_exact_abi, require_same_tensor_contract,
	scalar_binary, scalar_builder, scalar_f32, scalar_finish, scalar_input, scalar_ternary, scalar_unary,
};

const MAX_I32_INDEX: u64 = 2_147_483_647;
const OPERATIONS: &[(&str, &str)] = &[
	("gpu_accuracy", "gpu-core/src/kernels.rs:4472"),
	("gpu_accuracy_into", "gpu-core/src/kernels.rs:2914"),
	("gpu_argmax_accuracy_into", "gpu-core/src/kernels.rs:2963"),
	("gpu_contrastive_loss", "gpu-core/src/losses.rs:328"),
	("gpu_cosine_embedding_loss", "gpu-core/src/losses.rs:270"),
	("gpu_has_nan", "gpu-core/src/math_ops.rs:456"),
	("gpu_hinge_loss", "gpu-core/src/losses.rs:246"),
	("gpu_isfinite_all", "gpu-core/src/math_ops.rs:476"),
	("gpu_kl_div_loss", "gpu-core/src/losses.rs:225"),
	("gpu_mean_all", "gpu-core/src/reductions.rs:275"),
	("gpu_mse_into", "gpu-core/src/kernels.rs:2888"),
	("gpu_reduce_mean_cols", "gpu-core/src/kernels.rs:5176"),
	("gpu_reduce_var_cols", "gpu-core/src/kernels.rs:5209"),
	("gpu_ss_res_into", "gpu-core/src/kernels.rs:2862"),
	("gpu_triplet_loss", "gpu-core/src/losses.rs:299"),
];

pub(super) fn supports(descriptor: OperationDescriptor) -> bool {
	OPERATIONS.contains(&(descriptor.symbol, descriptor.source))
}

pub(super) fn dispatch(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> FamilyDispatch {
	let result = match supports(request.descriptor) {
		false => return FamilyDispatch::NotOwned,
		true => match request.descriptor.symbol {
			"gpu_accuracy" => emit_multiclass_correct_count(request, emitter),
			"gpu_accuracy_into" => emit_binary_accuracy(request, emitter),
			"gpu_argmax_accuracy_into" => emit_dense_multiclass_accuracy(request, emitter),
			"gpu_contrastive_loss" => emit_contrastive_loss(request, emitter),
			"gpu_cosine_embedding_loss" => emit_cosine_embedding_loss(request, emitter),
			"gpu_has_nan" => emit_finite_metric(request, emitter, false),
			"gpu_hinge_loss" => emit_hinge_loss(request, emitter),
			"gpu_isfinite_all" => emit_finite_metric(request, emitter, true),
			"gpu_kl_div_loss" => emit_kl_divergence_loss(request, emitter),
			"gpu_mean_all" => emit_global_mean(request, emitter),
			"gpu_mse_into" => emit_mean_squared_error(request, emitter),
			"gpu_reduce_mean_cols" => emit_column_mean(request, emitter),
			"gpu_reduce_var_cols" => emit_column_variance(request, emitter),
			"gpu_ss_res_into" => emit_sum_squared_residuals(request, emitter),
			"gpu_triplet_loss" => emit_triplet_loss(request, emitter),
			symbol => Err(request_error(
				request,
				format!("loss/metric dispatch is incomplete for {symbol}"),
			)),
		},
	};
	FamilyDispatch::Owned(result)
}

fn emit_binary_accuracy(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(
		request,
		&["predictions", "targets"],
		&["accuracy"],
		&["tree_lanes"],
	)?;
	let predictions = input(request, "predictions")?;
	let targets = input(request, "targets")?;
	let accuracy = output(request, "accuracy")?;
	require_equal_nonempty_f32(request, predictions, [("targets", targets)])?;
	require_f32_scalar(request, accuracy, "accuracy")?;
	let divisor = exact_f32_extent(request, predictions.shape.elements(), "element count")?;
	let tree_lanes = prepared_tree_lanes(request)?;
	let matches = emitter.intermediate(DType::I32, predictions.shape.clone())?;
	let match_count = emitter.intermediate(DType::I32, scalar_shape(request)?)?;
	emitter.emit(
		vec![predictions.id, targets.id],
		vec![matches],
		PrimitiveKind::Elementwise(Elementwise {
			program: binary_accuracy_program(request.descriptor.id)?,
		}),
	)?;
	emitter.emit(
		vec![matches],
		vec![match_count],
		reduction(
			request,
			ReduceOperator::Sum,
			all_axes(request, &predictions.shape)?,
			false,
			ReduceResult::Value,
			tree_lanes,
		)?,
	)?;
	emitter.emit(
		vec![match_count],
		vec![accuracy.id],
		PrimitiveKind::Elementwise(Elementwise {
			program: normalize_i32_count_program(request.descriptor.id, divisor)?,
		}),
	)
}

fn emit_multiclass_correct_count(
	request: &MaterializationRequest<'_>,
	emitter: &mut Emitter<'_>,
) -> OperationResult<()> {
	require_exact_abi(
		request,
		&["predictions", "targets"],
		&["correct_count"],
		&["tree_lanes"],
	)?;
	let predictions = input(request, "predictions")?;
	let targets = input(request, "targets")?;
	let correct_count = output(request, "correct_count")?;
	let (rows, classes) = require_f32_matrix(request, predictions, "predictions")?;
	require_dtype(request, targets, DType::I32, "targets")?;
	require_shape(request, targets, &[rows], "targets")?;
	require_f32_scalar(request, correct_count, "correct_count")?;
	exact_f32_extent(request, rows, "row count")?;
	let classes_i32 = match i32::try_from(classes) {
		Ok(value) => value,
		Err(conversion_error) => return Err(request_error(request, conversion_error.to_string())),
	};
	let tree_lanes = prepared_tree_lanes(request)?;
	let row_shape =
		Shape::new(vec![rows]).map_err(|error| language_error(request.descriptor.id, error.to_string()))?;
	let predicted_classes = emitter.intermediate(DType::I32, row_shape.clone())?;
	let matches = emitter.intermediate(DType::F32, row_shape)?;
	emitter.emit(
		vec![predictions.id],
		vec![predicted_classes],
		reduction(
			request,
			ReduceOperator::Maximum,
			axis(request, 1)?,
			false,
			ReduceResult::Index,
			tree_lanes,
		)?,
	)?;
	emitter.emit(
		vec![predicted_classes, targets.id],
		vec![matches],
		PrimitiveKind::Elementwise(Elementwise {
			program: checked_class_match_program(request.descriptor.id, classes_i32)?,
		}),
	)?;
	emitter.emit(
		vec![matches],
		vec![correct_count.id],
		reduction(
			request,
			ReduceOperator::Sum,
			axis(request, 0)?,
			false,
			ReduceResult::Value,
			tree_lanes,
		)?,
	)
}

fn emit_dense_multiclass_accuracy(
	request: &MaterializationRequest<'_>,
	emitter: &mut Emitter<'_>,
) -> OperationResult<()> {
	require_exact_abi(
		request,
		&["predictions", "targets"],
		&["accuracy"],
		&["tree_lanes"],
	)?;
	let predictions = input(request, "predictions")?;
	let targets = input(request, "targets")?;
	let accuracy = output(request, "accuracy")?;
	let (rows, _) = require_f32_matrix(request, predictions, "predictions")?;
	require_same_tensor_contract(request, predictions, targets, "targets")?;
	require_f32_scalar(request, accuracy, "accuracy")?;
	let divisor = exact_f32_extent(request, rows, "row count")?;
	let tree_lanes = prepared_tree_lanes(request)?;
	let row_shape =
		Shape::new(vec![rows]).map_err(|error| language_error(request.descriptor.id, error.to_string()))?;
	let predicted_classes = emitter.intermediate(DType::I32, row_shape.clone())?;
	let target_classes = emitter.intermediate(DType::I32, row_shape.clone())?;
	let matches = emitter.intermediate(DType::F32, row_shape)?;
	let match_count = emitter.intermediate(DType::F32, scalar_shape(request)?)?;
	let row_maximum = |request: &MaterializationRequest<'_>| {
		reduction(
			request,
			ReduceOperator::Maximum,
			axis(request, 1)?,
			false,
			ReduceResult::Index,
			tree_lanes,
		)
	};
	emitter.emit(
		vec![predictions.id],
		vec![predicted_classes],
		row_maximum(request)?,
	)?;
	emitter.emit(
		vec![targets.id],
		vec![target_classes],
		row_maximum(request)?,
	)?;
	emitter.emit(
		vec![predicted_classes, target_classes],
		vec![matches],
		PrimitiveKind::Elementwise(Elementwise {
			program: class_match_program(request.descriptor.id)?,
		}),
	)?;
	emitter.emit(
		vec![matches],
		vec![match_count],
		reduction(
			request,
			ReduceOperator::Sum,
			axis(request, 0)?,
			false,
			ReduceResult::Value,
			tree_lanes,
		)?,
	)?;
	emitter.emit(
		vec![match_count],
		vec![accuracy.id],
		PrimitiveKind::Elementwise(Elementwise {
			program: normalize_f32_count_program(request.descriptor.id, divisor)?,
		}),
	)
}

fn emit_finite_metric(
	request: &MaterializationRequest<'_>,
	emitter: &mut Emitter<'_>,
	all_finite: bool,
) -> OperationResult<()> {
	let output_name = match all_finite {
		true => "all_finite",
		false => "has_nan",
	};
	require_exact_abi(request, &["values"], &[output_name], &["tree_lanes"])?;
	let values = input(request, "values")?;
	let result = output(request, output_name)?;
	require_nonempty_f32(request, values, "values")?;
	require_i32_scalar(request, result, output_name)?;
	let tree_lanes = prepared_tree_lanes(request)?;
	let flags = emitter.intermediate(DType::I32, values.shape.clone())?;
	emitter.emit(
		vec![values.id],
		vec![flags],
		PrimitiveKind::Elementwise(Elementwise {
			program: finite_flag_program(request.descriptor.id, all_finite)?,
		}),
	)?;
	emitter.emit(
		vec![flags],
		vec![result.id],
		reduction(
			request,
			match all_finite {
				true => ReduceOperator::All,
				false => ReduceOperator::Any,
			},
			all_axes(request, &values.shape)?,
			false,
			ReduceResult::Value,
			tree_lanes,
		)?,
	)
}

fn emit_hinge_loss(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(
		request,
		&["scores", "labels"],
		&["losses", "gradients"],
		&[],
	)?;
	let scores = input(request, "scores")?;
	let labels = input(request, "labels")?;
	let losses = output(request, "losses")?;
	let gradients = output(request, "gradients")?;
	require_equal_nonempty_f32(
		request,
		scores,
		[
			("labels", labels),
			("losses", losses),
			("gradients", gradients),
		],
	)?;
	emitter.emit(
		vec![scores.id, labels.id],
		vec![losses.id, gradients.id],
		PrimitiveKind::Elementwise(Elementwise {
			program: hinge_loss_program(request.descriptor.id)?,
		}),
	)
}

fn emit_kl_divergence_loss(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(request, &["log_probabilities", "targets"], &["losses"], &[])?;
	let log_probabilities = input(request, "log_probabilities")?;
	let targets = input(request, "targets")?;
	let losses = output(request, "losses")?;
	require_equal_nonempty_f32(
		request,
		log_probabilities,
		[("targets", targets), ("losses", losses)],
	)?;
	let safe_targets = emitter.intermediate(DType::F32, targets.shape.clone())?;
	let logarithms = emitter.intermediate(DType::F32, targets.shape.clone())?;
	let logarithm = recipe_core::ScalarProgram::try_from(MathFunction::Log)
		.map_err(|error| language_error(request.descriptor.id, error.to_string()))?;
	emitter.emit_stage([
		KernelEmission {
			inputs: vec![targets.id],
			outputs: vec![safe_targets],
			kind: PrimitiveKind::Elementwise(Elementwise {
				program: positive_or_one_program(request.descriptor.id)?,
			}),
		},
		KernelEmission {
			inputs: vec![safe_targets],
			outputs: vec![logarithms],
			kind: PrimitiveKind::Elementwise(Elementwise { program: logarithm }),
		},
		KernelEmission {
			inputs: vec![log_probabilities.id, targets.id, logarithms],
			outputs: vec![losses.id],
			kind: PrimitiveKind::Elementwise(Elementwise {
				program: kl_divergence_program(request.descriptor.id)?,
			}),
		},
	])
}

fn emit_global_mean(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(request, &["values"], &["mean"], &["tree_lanes"])?;
	let values = input(request, "values")?;
	let mean = output(request, "mean")?;
	require_nonempty_f32(request, values, "values")?;
	require_f32_scalar(request, mean, "mean")?;
	let divisor = exact_f32_extent(request, values.shape.elements(), "element count")?;
	let tree_lanes = prepared_tree_lanes(request)?;
	let sum = emitter.intermediate(DType::F32, scalar_shape(request)?)?;
	emitter.emit(
		vec![values.id],
		vec![sum],
		reduction(
			request,
			ReduceOperator::Sum,
			all_axes(request, &values.shape)?,
			false,
			ReduceResult::Value,
			tree_lanes,
		)?,
	)?;
	emitter.emit(
		vec![sum],
		vec![mean.id],
		PrimitiveKind::Elementwise(Elementwise {
			program: normalize_f32_count_program(request.descriptor.id, divisor)?,
		}),
	)
}

fn emit_mean_squared_error(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(
		request,
		&["predictions", "targets"],
		&["mean_squared_error"],
		&["tree_lanes"],
	)?;
	let predictions = input(request, "predictions")?;
	let targets = input(request, "targets")?;
	let mean_squared_error = output(request, "mean_squared_error")?;
	require_equal_nonempty_f32(request, predictions, [("targets", targets)])?;
	require_f32_scalar(request, mean_squared_error, "mean_squared_error")?;
	let divisor = exact_f32_extent(request, predictions.shape.elements(), "element count")?;
	let tree_lanes = prepared_tree_lanes(request)?;
	let squared_residuals = emitter.intermediate(DType::F32, predictions.shape.clone())?;
	let residual_sum = emitter.intermediate(DType::F32, scalar_shape(request)?)?;
	emitter.emit(
		vec![predictions.id, targets.id],
		vec![squared_residuals],
		PrimitiveKind::Elementwise(Elementwise {
			program: squared_difference_program(request.descriptor.id)?,
		}),
	)?;
	emitter.emit(
		vec![squared_residuals],
		vec![residual_sum],
		reduction(
			request,
			ReduceOperator::Sum,
			all_axes(request, &predictions.shape)?,
			false,
			ReduceResult::Value,
			tree_lanes,
		)?,
	)?;
	emitter.emit(
		vec![residual_sum],
		vec![mean_squared_error.id],
		PrimitiveKind::Elementwise(Elementwise {
			program: normalize_f32_count_program(request.descriptor.id, divisor)?,
		}),
	)
}

fn emit_sum_squared_residuals(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(
		request,
		&["predictions", "targets"],
		&["sum_squared_residuals"],
		&["tree_lanes"],
	)?;
	let predictions = input(request, "predictions")?;
	let targets = input(request, "targets")?;
	let sum_squared_residuals = output(request, "sum_squared_residuals")?;
	require_equal_nonempty_f32(request, predictions, [("targets", targets)])?;
	require_f32_scalar(request, sum_squared_residuals, "sum_squared_residuals")?;
	let tree_lanes = prepared_tree_lanes(request)?;
	let squared_residuals = emitter.intermediate(DType::F32, predictions.shape.clone())?;
	emitter.emit(
		vec![predictions.id, targets.id],
		vec![squared_residuals],
		PrimitiveKind::Elementwise(Elementwise {
			program: squared_difference_program(request.descriptor.id)?,
		}),
	)?;
	emitter.emit(
		vec![squared_residuals],
		vec![sum_squared_residuals.id],
		reduction(
			request,
			ReduceOperator::Sum,
			all_axes(request, &predictions.shape)?,
			false,
			ReduceResult::Value,
			tree_lanes,
		)?,
	)
}

fn emit_column_mean(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(request, &["values"], &["means"], &["tree_lanes"])?;
	let values = input(request, "values")?;
	let means = output(request, "means")?;
	let (rows, columns) = require_f32_matrix(request, values, "values")?;
	require_dtype(request, means, DType::F32, "means")?;
	require_shape(request, means, &[columns], "means")?;
	let divisor = exact_f32_extent(request, rows, "row count")?;
	let tree_lanes = prepared_tree_lanes(request)?;
	let sums = emitter.intermediate(
		DType::F32,
		Shape::new(vec![columns]).map_err(|error| language_error(request.descriptor.id, error.to_string()))?,
	)?;
	emitter.emit(
		vec![values.id],
		vec![sums],
		reduction(
			request,
			ReduceOperator::Sum,
			axis(request, 0)?,
			false,
			ReduceResult::Value,
			tree_lanes,
		)?,
	)?;
	emitter.emit(
		vec![sums],
		vec![means.id],
		PrimitiveKind::Elementwise(Elementwise {
			program: normalize_f32_count_program(request.descriptor.id, divisor)?,
		}),
	)
}

fn emit_column_variance(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(request, &["values"], &["variances"], &["tree_lanes"])?;
	let values = input(request, "values")?;
	let variances = output(request, "variances")?;
	let (rows, columns) = require_f32_matrix(request, values, "values")?;
	require_dtype(request, variances, DType::F32, "variances")?;
	require_shape(request, variances, &[columns], "variances")?;
	let divisor = exact_f32_extent(request, rows, "row count")?;
	let tree_lanes = prepared_tree_lanes(request)?;
	let column_shape =
		Shape::new(vec![columns]).map_err(|error| language_error(request.descriptor.id, error.to_string()))?;
	let kept_column_shape =
		Shape::new(vec![1, columns]).map_err(|error| language_error(request.descriptor.id, error.to_string()))?;
	let sums = emitter.intermediate(DType::F32, column_shape)?;
	let squared_deviations = emitter.intermediate(DType::F32, values.shape.clone())?;
	let deviation_sums = emitter.intermediate(DType::F32, kept_column_shape.clone())?;
	let normalized = emitter.intermediate(DType::F32, kept_column_shape)?;
	emitter.emit(
		vec![values.id],
		vec![sums],
		reduction(
			request,
			ReduceOperator::Sum,
			axis(request, 0)?,
			false,
			ReduceResult::Value,
			tree_lanes,
		)?,
	)?;
	emitter.emit(
		vec![values.id, sums],
		vec![squared_deviations],
		PrimitiveKind::Elementwise(Elementwise {
			program: squared_deviation_program(request.descriptor.id, divisor)?,
		}),
	)?;
	emitter.emit_stage([
		KernelEmission {
			inputs: vec![squared_deviations],
			outputs: vec![deviation_sums],
			kind: reduction(
				request,
				ReduceOperator::Sum,
				axis(request, 0)?,
				true,
				ReduceResult::Value,
				tree_lanes,
			)?,
		},
		KernelEmission {
			inputs: vec![deviation_sums],
			outputs: vec![normalized],
			kind: PrimitiveKind::Elementwise(Elementwise {
				program: normalize_f32_count_program(request.descriptor.id, divisor)?,
			}),
		},
		KernelEmission {
			inputs: vec![normalized],
			outputs: vec![variances.id],
			kind: reduction(
				request,
				ReduceOperator::Sum,
				axis(request, 0)?,
				false,
				ReduceResult::Value,
				tree_lanes,
			)?,
		},
	])
}

fn emit_contrastive_loss(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(
		request,
		&["left", "right", "labels"],
		&["losses"],
		&["margin", "tree_lanes"],
	)?;
	let left = input(request, "left")?;
	let right = input(request, "right")?;
	let labels = input(request, "labels")?;
	let losses = output(request, "losses")?;
	let (rows, _) = require_paired_loss_tensors(request, left, right, labels, losses)?;
	let margin = finite_parameter(request, "margin")?;
	let tree_lanes = prepared_tree_lanes(request)?;
	let squared_differences = emitter.intermediate(DType::F32, left.shape.clone())?;
	let row_shape =
		Shape::new(vec![rows]).map_err(|error| language_error(request.descriptor.id, error.to_string()))?;
	let squared_distances = emitter.intermediate(DType::F32, row_shape)?;
	emitter.emit(
		vec![left.id, right.id],
		vec![squared_differences],
		PrimitiveKind::Elementwise(Elementwise {
			program: squared_difference_program(request.descriptor.id)?,
		}),
	)?;
	emitter.emit(
		vec![squared_differences],
		vec![squared_distances],
		reduction(
			request,
			ReduceOperator::Sum,
			axis(request, 1)?,
			false,
			ReduceResult::Value,
			tree_lanes,
		)?,
	)?;
	emitter.emit(
		vec![squared_distances, labels.id],
		vec![losses.id],
		PrimitiveKind::Elementwise(Elementwise {
			program: contrastive_loss_program(request.descriptor.id, margin)?,
		}),
	)
}

fn emit_cosine_embedding_loss(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(
		request,
		&["left", "right", "labels"],
		&["losses"],
		&["margin", "tree_lanes"],
	)?;
	let left = input(request, "left")?;
	let right = input(request, "right")?;
	let labels = input(request, "labels")?;
	let losses = output(request, "losses")?;
	let (rows, _) = require_paired_loss_tensors(request, left, right, labels, losses)?;
	let margin = finite_parameter(request, "margin")?;
	let tree_lanes = prepared_tree_lanes(request)?;
	let dot_terms = emitter.intermediate(DType::F32, left.shape.clone())?;
	let left_norm_terms = emitter.intermediate(DType::F32, left.shape.clone())?;
	let right_norm_terms = emitter.intermediate(DType::F32, left.shape.clone())?;
	let row_shape =
		Shape::new(vec![rows]).map_err(|error| language_error(request.descriptor.id, error.to_string()))?;
	let dot_products = emitter.intermediate(DType::F32, row_shape.clone())?;
	let left_norms_squared = emitter.intermediate(DType::F32, row_shape.clone())?;
	let right_norms_squared = emitter.intermediate(DType::F32, row_shape)?;
	emitter.emit(
		vec![left.id, right.id],
		vec![dot_terms, left_norm_terms, right_norm_terms],
		PrimitiveKind::Elementwise(Elementwise {
			program: cosine_terms_program(request.descriptor.id)?,
		}),
	)?;
	let row_sum = |request: &MaterializationRequest<'_>| {
		reduction(
			request,
			ReduceOperator::Sum,
			axis(request, 1)?,
			false,
			ReduceResult::Value,
			tree_lanes,
		)
	};
	emitter.emit(vec![dot_terms], vec![dot_products], row_sum(request)?)?;
	emitter.emit(
		vec![left_norm_terms],
		vec![left_norms_squared],
		row_sum(request)?,
	)?;
	emitter.emit(
		vec![right_norm_terms],
		vec![right_norms_squared],
		row_sum(request)?,
	)?;
	emitter.emit(
		vec![
			dot_products,
			left_norms_squared,
			right_norms_squared,
			labels.id,
		],
		vec![losses.id],
		PrimitiveKind::Elementwise(Elementwise {
			program: cosine_embedding_loss_program(request.descriptor.id, margin)?,
		}),
	)
}

fn emit_triplet_loss(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(
		request,
		&["anchor", "positive", "negative"],
		&["losses"],
		&["margin", "tree_lanes"],
	)?;
	let anchor = input(request, "anchor")?;
	let positive = input(request, "positive")?;
	let negative = input(request, "negative")?;
	let losses = output(request, "losses")?;
	let (rows, _) = require_f32_matrix(request, anchor, "anchor")?;
	require_same_tensor_contract(request, anchor, positive, "positive")?;
	require_same_tensor_contract(request, anchor, negative, "negative")?;
	require_dtype(request, losses, DType::F32, "losses")?;
	require_shape(request, losses, &[rows], "losses")?;
	let margin = finite_parameter(request, "margin")?;
	let tree_lanes = prepared_tree_lanes(request)?;
	let positive_terms = emitter.intermediate(DType::F32, anchor.shape.clone())?;
	let negative_terms = emitter.intermediate(DType::F32, anchor.shape.clone())?;
	let row_shape =
		Shape::new(vec![rows]).map_err(|error| language_error(request.descriptor.id, error.to_string()))?;
	let positive_distances = emitter.intermediate(DType::F32, row_shape.clone())?;
	let negative_distances = emitter.intermediate(DType::F32, row_shape)?;
	emitter.emit(
		vec![anchor.id, positive.id, negative.id],
		vec![positive_terms, negative_terms],
		PrimitiveKind::Elementwise(Elementwise {
			program: triplet_terms_program(request.descriptor.id)?,
		}),
	)?;
	let row_sum = |request: &MaterializationRequest<'_>| {
		reduction(
			request,
			ReduceOperator::Sum,
			axis(request, 1)?,
			false,
			ReduceResult::Value,
			tree_lanes,
		)
	};
	emitter.emit(
		vec![positive_terms],
		vec![positive_distances],
		row_sum(request)?,
	)?;
	emitter.emit(
		vec![negative_terms],
		vec![negative_distances],
		row_sum(request)?,
	)?;
	emitter.emit(
		vec![positive_distances, negative_distances],
		vec![losses.id],
		PrimitiveKind::Elementwise(Elementwise {
			program: triplet_loss_program(request.descriptor.id, margin)?,
		}),
	)
}

fn require_nonempty_f32(request: &MaterializationRequest<'_>, tensor: &Tensor, name: &str) -> OperationResult<()> {
	require_dtype(request, tensor, DType::F32, name)?;
	let elements = tensor.shape.elements();
	match elements == 0 || elements > MAX_I32_INDEX {
		true => Err(request_error(
			request,
			format!("{name} element count must be in 1..={MAX_I32_INDEX}"),
		)),
		false => Ok(()),
	}
}

fn require_equal_nonempty_f32<'a>(
	request: &MaterializationRequest<'_>,
	reference: &Tensor,
	others: impl IntoIterator<Item = (&'a str, &'a Tensor)>,
) -> OperationResult<()> {
	require_nonempty_f32(request, reference, "reference tensor")?;
	for (name, tensor) in others {
		require_same_tensor_contract(request, reference, tensor, name)?;
	}
	Ok(())
}

fn require_f32_matrix(
	request: &MaterializationRequest<'_>,
	tensor: &Tensor,
	name: &str,
) -> OperationResult<(u64, u64)> {
	require_nonempty_f32(request, tensor, name)?;
	match tensor.shape.rank() == 2 {
		true => {
			let rows = tensor.shape.extents()[0];
			let columns = tensor.shape.extents()[1];
			match rows == 0 || columns == 0 || rows > MAX_I32_INDEX || columns > MAX_I32_INDEX {
				true => Err(request_error(
					request,
					format!("{name} dimensions must each be in 1..={MAX_I32_INDEX}"),
				)),
				false => Ok((rows, columns)),
			}
		}
		false => Err(request_error(
			request,
			format!("{name} must be rank-two [rows, columns]"),
		)),
	}
}

fn require_paired_loss_tensors(
	request: &MaterializationRequest<'_>,
	left: &Tensor,
	right: &Tensor,
	labels: &Tensor,
	losses: &Tensor,
) -> OperationResult<(u64, u64)> {
	let (rows, columns) = require_f32_matrix(request, left, "left")?;
	require_same_tensor_contract(request, left, right, "right")?;
	require_dtype(request, labels, DType::F32, "labels")?;
	require_shape(request, labels, &[rows], "labels")?;
	require_dtype(request, losses, DType::F32, "losses")?;
	require_shape(request, losses, &[rows], "losses")?;
	Ok((rows, columns))
}

fn require_f32_scalar(request: &MaterializationRequest<'_>, tensor: &Tensor, name: &str) -> OperationResult<()> {
	require_dtype(request, tensor, DType::F32, name)?;
	require_shape(request, tensor, &[1], name)
}

fn require_i32_scalar(request: &MaterializationRequest<'_>, tensor: &Tensor, name: &str) -> OperationResult<()> {
	require_dtype(request, tensor, DType::I32, name)?;
	require_shape(request, tensor, &[1], name)
}

fn require_shape(
	request: &MaterializationRequest<'_>,
	tensor: &Tensor,
	shape: &[u64],
	name: &str,
) -> OperationResult<()> {
	match tensor.shape.extents() == shape {
		true => Ok(()),
		false => Err(request_error(
			request,
			format!(
				"{name} has shape {:?}, expected {shape:?}",
				tensor.shape.extents()
			),
		)),
	}
}

fn finite_parameter(request: &MaterializationRequest<'_>, name: &str) -> OperationResult<f32> {
	let value = prepared_f32(request, name)?;
	match value.is_finite() {
		true => Ok(value),
		false => Err(request_error(
			request,
			format!("{name} must be a finite f32"),
		)),
	}
}

fn scalar_shape(request: &MaterializationRequest<'_>) -> OperationResult<Shape> {
	Shape::new(vec![1]).map_err(|error| language_error(request.descriptor.id, error.to_string()))
}

fn axis(request: &MaterializationRequest<'_>, value: usize) -> OperationResult<AxisSet> {
	AxisSet::new(vec![value]).map_err(|error| language_error(request.descriptor.id, error.to_string()))
}

fn all_axes(request: &MaterializationRequest<'_>, shape: &Shape) -> OperationResult<AxisSet> {
	AxisSet::new((0..shape.rank()).collect())
		.map_err(|error| language_error(request.descriptor.id, error.to_string()))
}

fn reduction(
	request: &MaterializationRequest<'_>,
	operator: ReduceOperator,
	axes: AxisSet,
	keep_dimensions: bool,
	result: ReduceResult,
	tree_lanes: u32,
) -> OperationResult<PrimitiveKind> {
	let kind = PrimitiveKind::Reduce(Reduce {
		operator,
		axes,
		keep_dimensions,
		result,
		tree_lanes,
	});
	match supports(request.descriptor) {
		true => Ok(kind),
		false => Err(request_error(
			request,
			"reduction requested by a descriptor outside the loss/metric family",
		)),
	}
}

fn binary_accuracy_program(operation: OperationId) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let prediction = scalar_input(operation, &mut builder, DType::F32)?;
	let target = scalar_input(operation, &mut builder, DType::F32)?;
	let threshold = scalar_f32(operation, &mut builder, 0.5)?;
	let predicted_positive = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::GreaterThanOrEqual,
		prediction,
		threshold,
	)?;
	let target_positive = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::GreaterThanOrEqual,
		target,
		threshold,
	)?;
	let matches = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Equal,
		predicted_positive,
		target_positive,
	)?;
	scalar_finish(operation, builder, &[matches])
}

fn normalize_i32_count_program(operation: OperationId, divisor: f32) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let count = scalar_input(operation, &mut builder, DType::I32)?;
	let count = scalar_unary(
		operation,
		&mut builder,
		ScalarOpcode::ConvertI32ToF32,
		count,
	)?;
	let divisor = scalar_f32(operation, &mut builder, divisor)?;
	let result = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Divide,
		count,
		divisor,
	)?;
	scalar_finish(operation, builder, &[result])
}

fn normalize_f32_count_program(operation: OperationId, divisor: f32) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let count = scalar_input(operation, &mut builder, DType::F32)?;
	let divisor = scalar_f32(operation, &mut builder, divisor)?;
	let result = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Divide,
		count,
		divisor,
	)?;
	scalar_finish(operation, builder, &[result])
}

fn checked_class_match_program(operation: OperationId, classes: i32) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let prediction = scalar_input(operation, &mut builder, DType::I32)?;
	let target = scalar_input(operation, &mut builder, DType::I32)?;
	let zero = builder
		.i32(0)
		.map_err(|error| language_error(operation, error.to_string()))?;
	let classes = builder
		.i32(classes)
		.map_err(|error| language_error(operation, error.to_string()))?;
	let nonnegative = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::GreaterThanOrEqual,
		target,
		zero,
	)?;
	scalar_unary(operation, &mut builder, ScalarOpcode::Require, nonnegative)?;
	let below_classes = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::LessThan,
		target,
		classes,
	)?;
	scalar_unary(
		operation,
		&mut builder,
		ScalarOpcode::Require,
		below_classes,
	)?;
	let matches = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Equal,
		prediction,
		target,
	)?;
	let matches = scalar_unary(
		operation,
		&mut builder,
		ScalarOpcode::ConvertI32ToF32,
		matches,
	)?;
	scalar_finish(operation, builder, &[matches])
}

fn class_match_program(operation: OperationId) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let prediction = scalar_input(operation, &mut builder, DType::I32)?;
	let target = scalar_input(operation, &mut builder, DType::I32)?;
	let matches = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Equal,
		prediction,
		target,
	)?;
	let matches = scalar_unary(
		operation,
		&mut builder,
		ScalarOpcode::ConvertI32ToF32,
		matches,
	)?;
	scalar_finish(operation, builder, &[matches])
}

fn finite_flag_program(operation: OperationId, finite: bool) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let value = scalar_input(operation, &mut builder, DType::F32)?;
	let flag = scalar_unary(
		operation,
		&mut builder,
		match finite {
			true => ScalarOpcode::IsFinite,
			false => ScalarOpcode::IsNan,
		},
		value,
	)?;
	scalar_finish(operation, builder, &[flag])
}

fn hinge_loss_program(operation: OperationId) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let score = scalar_input(operation, &mut builder, DType::F32)?;
	let label = scalar_input(operation, &mut builder, DType::F32)?;
	let one = scalar_f32(operation, &mut builder, 1.0)?;
	let zero = scalar_f32(operation, &mut builder, 0.0)?;
	let product = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		label,
		score,
	)?;
	let margin = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Subtract,
		one,
		product,
	)?;
	let active = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::GreaterThan,
		margin,
		zero,
	)?;
	let loss = scalar_ternary(
		operation,
		&mut builder,
		ScalarOpcode::Select,
		active,
		margin,
		zero,
	)?;
	let negative_label = scalar_unary(operation, &mut builder, ScalarOpcode::Negate, label)?;
	let gradient = scalar_ternary(
		operation,
		&mut builder,
		ScalarOpcode::Select,
		active,
		negative_label,
		zero,
	)?;
	scalar_finish(operation, builder, &[loss, gradient])
}

fn positive_or_one_program(operation: OperationId) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let target = scalar_input(operation, &mut builder, DType::F32)?;
	let zero = scalar_f32(operation, &mut builder, 0.0)?;
	let one = scalar_f32(operation, &mut builder, 1.0)?;
	let positive = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::GreaterThan,
		target,
		zero,
	)?;
	let safe = scalar_ternary(
		operation,
		&mut builder,
		ScalarOpcode::Select,
		positive,
		target,
		one,
	)?;
	scalar_finish(operation, builder, &[safe])
}

fn kl_divergence_program(operation: OperationId) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let log_probability = scalar_input(operation, &mut builder, DType::F32)?;
	let target = scalar_input(operation, &mut builder, DType::F32)?;
	let target_logarithm = scalar_input(operation, &mut builder, DType::F32)?;
	let zero = scalar_f32(operation, &mut builder, 0.0)?;
	let positive = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::GreaterThan,
		target,
		zero,
	)?;
	let difference = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Subtract,
		target_logarithm,
		log_probability,
	)?;
	let positive_loss = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		target,
		difference,
	)?;
	let loss = scalar_ternary(
		operation,
		&mut builder,
		ScalarOpcode::Select,
		positive,
		positive_loss,
		zero,
	)?;
	scalar_finish(operation, builder, &[loss])
}

fn squared_difference_program(operation: OperationId) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let left = scalar_input(operation, &mut builder, DType::F32)?;
	let right = scalar_input(operation, &mut builder, DType::F32)?;
	let difference = scalar_binary(operation, &mut builder, ScalarOpcode::Subtract, left, right)?;
	let square = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		difference,
		difference,
	)?;
	scalar_finish(operation, builder, &[square])
}

fn squared_deviation_program(operation: OperationId, divisor: f32) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let value = scalar_input(operation, &mut builder, DType::F32)?;
	let sum = scalar_input(operation, &mut builder, DType::F32)?;
	let divisor = scalar_f32(operation, &mut builder, divisor)?;
	let mean = scalar_binary(operation, &mut builder, ScalarOpcode::Divide, sum, divisor)?;
	let difference = scalar_binary(operation, &mut builder, ScalarOpcode::Subtract, value, mean)?;
	let square = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		difference,
		difference,
	)?;
	scalar_finish(operation, builder, &[square])
}

fn contrastive_loss_program(operation: OperationId, margin: f32) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let distance_squared = scalar_input(operation, &mut builder, DType::F32)?;
	let label = scalar_input(operation, &mut builder, DType::F32)?;
	require_finite_nonnegative(operation, &mut builder, distance_squared)?;
	let distance = scalar_unary(
		operation,
		&mut builder,
		ScalarOpcode::SquareRoot,
		distance_squared,
	)?;
	let margin = scalar_f32(operation, &mut builder, margin)?;
	let zero = scalar_f32(operation, &mut builder, 0.0)?;
	let one = scalar_f32(operation, &mut builder, 1.0)?;
	let remaining_margin = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Subtract,
		margin,
		distance,
	)?;
	let active = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::GreaterThan,
		remaining_margin,
		zero,
	)?;
	let squared_margin = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		remaining_margin,
		remaining_margin,
	)?;
	let negative_term = scalar_ternary(
		operation,
		&mut builder,
		ScalarOpcode::Select,
		active,
		squared_margin,
		zero,
	)?;
	let positive_loss = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		label,
		distance_squared,
	)?;
	let negative_weight = scalar_binary(operation, &mut builder, ScalarOpcode::Subtract, one, label)?;
	let negative_loss = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		negative_weight,
		negative_term,
	)?;
	let loss = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Add,
		positive_loss,
		negative_loss,
	)?;
	scalar_finish(operation, builder, &[loss])
}

fn cosine_terms_program(operation: OperationId) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let left = scalar_input(operation, &mut builder, DType::F32)?;
	let right = scalar_input(operation, &mut builder, DType::F32)?;
	let dot = scalar_binary(operation, &mut builder, ScalarOpcode::Multiply, left, right)?;
	let left_square = scalar_binary(operation, &mut builder, ScalarOpcode::Multiply, left, left)?;
	let right_square = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		right,
		right,
	)?;
	scalar_finish(operation, builder, &[dot, left_square, right_square])
}

fn cosine_embedding_loss_program(operation: OperationId, margin: f32) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let dot = scalar_input(operation, &mut builder, DType::F32)?;
	let left_norm_squared = scalar_input(operation, &mut builder, DType::F32)?;
	let right_norm_squared = scalar_input(operation, &mut builder, DType::F32)?;
	let label = scalar_input(operation, &mut builder, DType::F32)?;
	require_finite(operation, &mut builder, dot)?;
	require_finite_nonnegative(operation, &mut builder, left_norm_squared)?;
	require_finite_nonnegative(operation, &mut builder, right_norm_squared)?;
	let left_norm = scalar_unary(
		operation,
		&mut builder,
		ScalarOpcode::SquareRoot,
		left_norm_squared,
	)?;
	let right_norm = scalar_unary(
		operation,
		&mut builder,
		ScalarOpcode::SquareRoot,
		right_norm_squared,
	)?;
	let norm_product = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		left_norm,
		right_norm,
	)?;
	let epsilon = scalar_f32(operation, &mut builder, 1.0e-12)?;
	let denominator = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Add,
		norm_product,
		epsilon,
	)?;
	let cosine = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Divide,
		dot,
		denominator,
	)?;
	let zero = scalar_f32(operation, &mut builder, 0.0)?;
	let one = scalar_f32(operation, &mut builder, 1.0)?;
	let positive_label = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::GreaterThan,
		label,
		zero,
	)?;
	let positive_loss = scalar_binary(operation, &mut builder, ScalarOpcode::Subtract, one, cosine)?;
	let margin = scalar_f32(operation, &mut builder, margin)?;
	let margin_difference = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Subtract,
		cosine,
		margin,
	)?;
	let margin_active = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::GreaterThan,
		margin_difference,
		zero,
	)?;
	let negative_loss = scalar_ternary(
		operation,
		&mut builder,
		ScalarOpcode::Select,
		margin_active,
		margin_difference,
		zero,
	)?;
	let loss = scalar_ternary(
		operation,
		&mut builder,
		ScalarOpcode::Select,
		positive_label,
		positive_loss,
		negative_loss,
	)?;
	scalar_finish(operation, builder, &[loss])
}

fn triplet_terms_program(operation: OperationId) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let anchor = scalar_input(operation, &mut builder, DType::F32)?;
	let positive = scalar_input(operation, &mut builder, DType::F32)?;
	let negative = scalar_input(operation, &mut builder, DType::F32)?;
	let positive_difference = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Subtract,
		anchor,
		positive,
	)?;
	let negative_difference = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Subtract,
		anchor,
		negative,
	)?;
	let positive_square = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		positive_difference,
		positive_difference,
	)?;
	let negative_square = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		negative_difference,
		negative_difference,
	)?;
	scalar_finish(operation, builder, &[positive_square, negative_square])
}

fn triplet_loss_program(operation: OperationId, margin: f32) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let positive_distance = scalar_input(operation, &mut builder, DType::F32)?;
	let negative_distance = scalar_input(operation, &mut builder, DType::F32)?;
	require_finite_nonnegative(operation, &mut builder, positive_distance)?;
	require_finite_nonnegative(operation, &mut builder, negative_distance)?;
	let difference = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Subtract,
		positive_distance,
		negative_distance,
	)?;
	let margin = scalar_f32(operation, &mut builder, margin)?;
	let with_margin = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Add,
		difference,
		margin,
	)?;
	let zero = scalar_f32(operation, &mut builder, 0.0)?;
	let active = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::GreaterThan,
		with_margin,
		zero,
	)?;
	let loss = scalar_ternary(
		operation,
		&mut builder,
		ScalarOpcode::Select,
		active,
		with_margin,
		zero,
	)?;
	scalar_finish(operation, builder, &[loss])
}

fn require_finite(
	operation: OperationId,
	builder: &mut ScalarProgramBuilder,
	value: recipe_language::ScalarExpression,
) -> OperationResult<()> {
	let finite = scalar_unary(operation, builder, ScalarOpcode::IsFinite, value)?;
	scalar_unary(operation, builder, ScalarOpcode::Require, finite)?;
	Ok(())
}

fn require_finite_nonnegative(
	operation: OperationId,
	builder: &mut ScalarProgramBuilder,
	value: recipe_language::ScalarExpression,
) -> OperationResult<()> {
	require_finite(operation, builder, value)?;
	let zero = scalar_f32(operation, builder, 0.0)?;
	let nonnegative = scalar_binary(
		operation,
		builder,
		ScalarOpcode::GreaterThanOrEqual,
		value,
		zero,
	)?;
	scalar_unary(operation, builder, ScalarOpcode::Require, nonnegative)?;
	Ok(())
}
