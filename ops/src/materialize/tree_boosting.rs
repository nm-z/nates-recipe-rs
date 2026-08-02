use recipe_core::{DType, ScalarOpcode};
use recipe_language::{
	AtomicOperation, AtomicOrdering, AxisSet, Elementwise, Gather, Histogram, IndexBounds, PrimitiveKind,
	RandomDistribution, RandomKey, RandomMap, Reduce, ReduceOperator, ReduceResult, Scan, ScanMode, Scatter,
	ScatterConflict, Shape, Sort, SortDirection, Tensor,
};

use super::{
	Emitter, FamilyDispatch, KernelEmission, MAX_I32_INDEX, MaterializationRequest, emit_tree_leaf_split,
	identity_program, input, language_error, operation_error, output, prepared_f32, prepared_tree_lanes,
	prepared_u64, request_error, require_dtype, require_exact_abi, require_shape, require_true, scalar_binary,
	scalar_builder, scalar_f32, scalar_finish, scalar_input, scalar_ternary, scalar_unary,
};
use crate::{OperationDescriptor, OperationErrorKind, OperationId, OperationResult};

const OPERATIONS: &[(&str, &str)] = &[
	("gpu_argmax_write_split", "gpu-core/src/kernels.rs:4155"),
	("gpu_bootstrap_sample", "gpu-core/src/forest.rs:58"),
	("gpu_feature_subset", "gpu-core/src/forest.rs:82"),
	("gpu_grad_hess_into", "gpu-core/src/kernels.rs:3703"),
	("gpu_histogram_build", "gpu-core/src/kernels.rs:6745"),
	("gpu_leaf_finalize", "gpu-core/src/kernels.rs:4372"),
	("gpu_leaf_reduce", "gpu-core/src/kernels.rs:4346"),
	("gpu_leaf_split_apply", "gpu-core/src/kernels.rs:7173"),
	("gpu_lgbm_best_split", "gpu-core/src/kernels.rs:7080"),
	("gpu_lgbm_hist_subtract", "gpu-core/src/kernels.rs:7053"),
	("gpu_lgbm_histogram", "gpu-core/src/kernels.rs:7018"),
	("gpu_lgbm_leaf_reduce", "gpu-core/src/kernels.rs:7119"),
	("gpu_oblivious_histogram", "gpu-core/src/kernels.rs:4205"),
	("gpu_oblivious_split_eval", "gpu-core/src/kernels.rs:4396"),
	("gpu_ordered_target_stats", "gpu-core/src/catboost.rs:100"),
	("gpu_random_threshold_split", "gpu-core/src/forest.rs:108"),
	("gpu_scatter_add_by_leaf", "gpu-core/src/kernels.rs:4323"),
	(
		"gpu_scatter_add_by_leaf_col",
		"gpu-core/src/kernels.rs:4497",
	),
	("gpu_split_eval", "gpu-core/src/kernels.rs:6778"),
	("gpu_tb_histogram", "gpu-core/src/kernels.rs:3737"),
	("gpu_tb_leaf_sum", "gpu-core/src/kernels.rs:3830"),
	("gpu_tb_leaf_val", "gpu-core/src/kernels.rs:3856"),
	("gpu_tb_split_eval", "gpu-core/src/kernels.rs:3770"),
	("gpu_write_split", "gpu-core/src/kernels.rs:4182"),
];

pub(super) fn supports(descriptor: OperationDescriptor) -> bool {
	OPERATIONS.contains(&(descriptor.symbol, descriptor.source))
}

pub(super) fn dispatch(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> FamilyDispatch {
	match supports(request.descriptor) {
		false => FamilyDispatch::NotOwned,
		true => {
			let result = match request.descriptor.symbol {
				"gpu_argmax_write_split" => emit_argmax_split(request, emitter),
				"gpu_bootstrap_sample" => emit_bootstrap_sample(request, emitter),
				"gpu_feature_subset" => emit_feature_subset(request, emitter),
				"gpu_grad_hess_into" => emit_gradient_hessian(request, emitter),
				"gpu_histogram_build" | "gpu_lgbm_histogram" => emit_tree_histogram(request, emitter, true),
				"gpu_leaf_finalize" | "gpu_tb_leaf_val" => emit_regularized_leaf(request, emitter, false),
				"gpu_leaf_reduce" => emit_leaf_reduce(request, emitter),
				"gpu_leaf_split_apply" => emit_tree_leaf_split(request, emitter),
				"gpu_lgbm_best_split" | "gpu_oblivious_split_eval" | "gpu_split_eval" | "gpu_tb_split_eval" => {
					emit_two_bin_split(request, emitter)
				}
				"gpu_lgbm_hist_subtract" => emit_histogram_subtraction(request, emitter),
				"gpu_lgbm_leaf_reduce" | "gpu_tb_leaf_sum" => emit_regularized_leaf(request, emitter, true),
				"gpu_oblivious_histogram" | "gpu_tb_histogram" => emit_tree_histogram(request, emitter, false),
				"gpu_ordered_target_stats" => emit_ordered_target_statistics(request, emitter),
				"gpu_random_threshold_split" => emit_random_threshold(request, emitter),
				"gpu_scatter_add_by_leaf" | "gpu_scatter_add_by_leaf_col" => {
					emit_leaf_prediction(request, emitter)
				}
				"gpu_write_split" => emit_write_split(request, emitter),
				symbol => {
					Err(operation_error(
						request.descriptor.id,
						OperationErrorKind::GraphMaterializationFailed,
						format!("tree/boosting dispatch is incomplete for {symbol}"),
					))
				}
			};
			FamilyDispatch::Owned(result)
		}
	}
}

fn emit_argmax_split(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(
		request,
		&["gains"],
		&["best_gain", "best_feature", "best_bin"],
		&["features", "bins", "tree_lanes"],
	)?;
	let gains = input(request, "gains")?;
	let best_gain = output(request, "best_gain")?;
	let best_feature = output(request, "best_feature")?;
	let best_bin = output(request, "best_bin")?;
	let features = prepared_dimension(request, "features")?;
	let bins = prepared_dimension(request, "bins")?;
	let tree_lanes = prepared_tree_lanes(request)?;
	require_dtype(request, gains, DType::F32, "gains")?;
	require_shape(request, gains, &[features, bins], "gains")?;
	require_scalar(request, best_gain, DType::F32, "best_gain")?;
	require_scalar(request, best_feature, DType::I32, "best_feature")?;
	require_scalar(request, best_bin, DType::I32, "best_bin")?;

	let best_index = emitter.intermediate(DType::I32, scalar_shape(request)?)?;
	emitter.emit(
		vec![gains.id],
		vec![best_gain.id, best_index],
		reduction(
			request,
			ReduceOperator::Maximum,
			&[0, 1],
			false,
			ReduceResult::ValueAndIndex,
			tree_lanes,
		)?,
	)?;
	emitter.emit(
		vec![best_index],
		vec![best_feature.id, best_bin.id],
		PrimitiveKind::Elementwise(Elementwise {
			program: split_index_program(request.descriptor.id, bins)?,
		}),
	)
}

fn emit_bootstrap_sample(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(request, &["row_ids"], &["sampled_rows"], &[
		"rows",
		"samples",
		"seed_low",
		"seed_high",
		"stream",
	])?;
	let row_ids = input(request, "row_ids")?;
	let sampled_rows = output(request, "sampled_rows")?;
	let rows = prepared_dimension(request, "rows")?;
	let samples = prepared_dimension(request, "samples")?;
	let key = prepared_random_key(request)?;
	require_dtype(request, row_ids, DType::I32, "row_ids")?;
	require_shape(request, row_ids, &[rows], "row_ids")?;
	require_dtype(request, sampled_rows, DType::I32, "sampled_rows")?;
	require_shape(request, sampled_rows, &[samples], "sampled_rows")?;
	let high_exclusive =
		i32::try_from(rows).map_err(|conversion_error| request_error(request, conversion_error.to_string()))?;

	let random_indices = emitter.intermediate(DType::I32, sampled_rows.shape.clone())?;
	emitter.emit(
		Vec::new(),
		vec![random_indices],
		PrimitiveKind::Random(RandomMap {
			distribution: RandomDistribution::UniformI32 {
				low: 0,
				high_exclusive,
			},
			key,
			philox_rounds: 10,
		}),
	)?;
	emitter.emit(
		vec![row_ids.id, random_indices],
		vec![sampled_rows.id],
		gather(),
	)
}

fn emit_feature_subset(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(
		request,
		&["feature_ids", "selection_indices"],
		&["selected_features"],
		&[
			"features",
			"selected",
			"selection_indices_verified",
			"selection_indices_unique",
		],
	)?;
	let feature_ids = input(request, "feature_ids")?;
	let selection_indices = input(request, "selection_indices")?;
	let selected_features = output(request, "selected_features")?;
	let features = prepared_dimension(request, "features")?;
	let selected = prepared_dimension(request, "selected")?;
	require_supported(
		request,
		selected <= features,
		"selected feature count exceeds the available feature count",
	)?;
	require_true(request, "selection_indices_verified")?;
	require_true(request, "selection_indices_unique")?;
	require_dtype(request, feature_ids, DType::I32, "feature_ids")?;
	require_shape(request, feature_ids, &[features], "feature_ids")?;
	require_dtype(request, selection_indices, DType::I32, "selection_indices")?;
	require_shape(request, selection_indices, &[selected], "selection_indices")?;
	require_dtype(request, selected_features, DType::I32, "selected_features")?;
	require_shape(request, selected_features, &[selected], "selected_features")?;

	let gathered = emitter.intermediate(DType::I32, selected_features.shape.clone())?;
	emitter.emit(
		vec![feature_ids.id, selection_indices.id],
		vec![gathered],
		gather(),
	)?;
	emitter.emit(
		vec![gathered],
		vec![selected_features.id],
		PrimitiveKind::Elementwise(Elementwise {
			program: identity_program(request.descriptor.id, DType::I32)?,
		}),
	)
}

fn emit_gradient_hessian(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(
		request,
		&["probabilities", "targets", "weights", "mask"],
		&["gradients", "hessians"],
		&["rows", "class_index"],
	)?;
	let probabilities = input(request, "probabilities")?;
	let targets = input(request, "targets")?;
	let weights = input(request, "weights")?;
	let mask = input(request, "mask")?;
	let gradients = output(request, "gradients")?;
	let hessians = output(request, "hessians")?;
	let rows = prepared_dimension(request, "rows")?;
	let class_index = prepared_i32(request, "class_index")?;
	for (name, tensor) in [
		("probabilities", probabilities),
		("weights", weights),
		("gradients", gradients),
		("hessians", hessians),
	] {
		require_dtype(request, tensor, DType::F32, name)?;
		require_shape(request, tensor, &[rows], name)?;
	}
	for (name, tensor) in [("targets", targets), ("mask", mask)] {
		require_dtype(request, tensor, DType::I32, name)?;
		require_shape(request, tensor, &[rows], name)?;
	}
	emitter.emit(
		vec![probabilities.id, targets.id, weights.id, mask.id],
		vec![gradients.id, hessians.id],
		PrimitiveKind::Elementwise(Elementwise {
			program: gradient_hessian_program(request.descriptor.id, class_index)?,
		}),
	)
}

fn emit_tree_histogram(
	request: &MaterializationRequest<'_>,
	emitter: &mut Emitter<'_>,
	include_counts: bool,
) -> OperationResult<()> {
	let output_names = match include_counts {
		true => &["gradient_histogram", "hessian_histogram", "count_histogram"][..],
		false => &["gradient_histogram", "hessian_histogram"][..],
	};
	require_exact_abi(
		request,
		&[
			"bin_indices",
			"gradient_contributions",
			"hessian_contributions",
		],
		output_names,
		&[
			"contributions",
			"histogram_bins",
			"contribution_table_verified",
		],
	)?;
	let bin_indices = input(request, "bin_indices")?;
	let gradients = input(request, "gradient_contributions")?;
	let hessians = input(request, "hessian_contributions")?;
	let gradient_histogram = output(request, "gradient_histogram")?;
	let hessian_histogram = output(request, "hessian_histogram")?;
	let count_histogram = match include_counts {
		true => Some(output(request, "count_histogram")?),
		false => None,
	};
	let contributions = prepared_dimension(request, "contributions")?;
	let bins = prepared_dimension(request, "histogram_bins")?;
	require_true(request, "contribution_table_verified")?;
	require_dtype(request, bin_indices, DType::I32, "bin_indices")?;
	require_shape(request, bin_indices, &[contributions], "bin_indices")?;
	for (name, tensor) in [
		("gradient_contributions", gradients),
		("hessian_contributions", hessians),
	] {
		require_dtype(request, tensor, DType::F32, name)?;
		require_shape(request, tensor, &[contributions], name)?;
	}
	for (name, tensor) in [
		("gradient_histogram", gradient_histogram),
		("hessian_histogram", hessian_histogram),
	] {
		require_dtype(request, tensor, DType::F32, name)?;
		require_shape(request, tensor, &[bins], name)?;
	}
	count_histogram.into_iter().try_for_each(|tensor| {
		require_dtype(request, tensor, DType::I32, "count_histogram")?;
		require_shape(request, tensor, &[bins], "count_histogram")
	})?;

	let mapped_bins = emitter.intermediate(DType::I32, bin_indices.shape.clone())?;
	let mapped_gradients = emitter.intermediate(DType::F32, gradients.shape.clone())?;
	let mapped_hessians = emitter.intermediate(DType::F32, hessians.shape.clone())?;
	emitter.emit(
		vec![bin_indices.id, gradients.id, hessians.id],
		vec![mapped_bins, mapped_gradients, mapped_hessians],
		PrimitiveKind::Elementwise(Elementwise {
			program: histogram_contribution_program(request.descriptor.id)?,
		}),
	)?;
	let histogram_bins =
		u32::try_from(bins).map_err(|conversion_error| request_error(request, conversion_error.to_string()))?;
	let weighted = |weights, destination| {
		KernelEmission {
			inputs: vec![mapped_bins, weights],
			outputs: vec![destination],
			kind: PrimitiveKind::Histogram(Histogram {
				bins: histogram_bins,
				weighted: true,
				ordering: AtomicOrdering::SequentiallyConsistent,
			}),
		}
	};
	let mut kernels = vec![
		weighted(mapped_gradients, gradient_histogram.id),
		weighted(mapped_hessians, hessian_histogram.id),
	];
	kernels.extend(count_histogram.map(|tensor| {
		KernelEmission {
			inputs: vec![mapped_bins],
			outputs: vec![tensor.id],
			kind: PrimitiveKind::Histogram(Histogram {
				bins: histogram_bins,
				weighted: false,
				ordering: AtomicOrdering::SequentiallyConsistent,
			}),
		}
	}));
	emitter.emit_stage(kernels)
}

fn emit_regularized_leaf(
	request: &MaterializationRequest<'_>,
	emitter: &mut Emitter<'_>,
	expose_sums: bool,
) -> OperationResult<()> {
	let output_names = match expose_sums {
		true => &["leaf_gradient", "leaf_hessian", "leaf_value"][..],
		false => &["leaf_value"][..],
	};
	require_exact_abi(
		request,
		&[
			"leaf_indices",
			"gradient_contributions",
			"hessian_contributions",
		],
		output_names,
		&[
			"contributions",
			"leaves",
			"lambda",
			"contribution_table_verified",
			"tree_lanes",
		],
	)?;
	let leaf_indices = input(request, "leaf_indices")?;
	let gradients = input(request, "gradient_contributions")?;
	let hessians = input(request, "hessian_contributions")?;
	let leaf_value = output(request, "leaf_value")?;
	let sum_outputs = match expose_sums {
		true => {
			Some((
				output(request, "leaf_gradient")?,
				output(request, "leaf_hessian")?,
			))
		}
		false => None,
	};
	let contributions = prepared_dimension(request, "contributions")?;
	let leaves = prepared_dimension(request, "leaves")?;
	require_supported(
		request,
		leaves == 1,
		"the exact finite regularized-leaf graph currently supports one leaf",
	)?;
	let lambda = finite_positive(request, "lambda")?;
	let tree_lanes = prepared_tree_lanes(request)?;
	require_true(request, "contribution_table_verified")?;
	require_dtype(request, leaf_indices, DType::I32, "leaf_indices")?;
	require_shape(request, leaf_indices, &[contributions], "leaf_indices")?;
	for (name, tensor) in [
		("gradient_contributions", gradients),
		("hessian_contributions", hessians),
	] {
		require_dtype(request, tensor, DType::F32, name)?;
		require_shape(request, tensor, &[contributions], name)?;
	}
	require_scalar(request, leaf_value, DType::F32, "leaf_value")?;
	sum_outputs
		.into_iter()
		.try_for_each(|(leaf_gradient, leaf_hessian)| {
			require_scalar(request, leaf_gradient, DType::F32, "leaf_gradient")?;
			require_scalar(request, leaf_hessian, DType::F32, "leaf_hessian")
		})?;

	let gradient_histogram = emitter.intermediate(DType::F32, scalar_shape(request)?)?;
	let hessian_histogram = emitter.intermediate(DType::F32, scalar_shape(request)?)?;
	let histogram = |weights, destination| {
		KernelEmission {
			inputs: vec![leaf_indices.id, weights],
			outputs: vec![destination],
			kind: PrimitiveKind::Histogram(Histogram {
				bins: 1,
				weighted: true,
				ordering: AtomicOrdering::SequentiallyConsistent,
			}),
		}
	};
	emitter.emit_stage([
		histogram(gradients.id, gradient_histogram),
		histogram(hessians.id, hessian_histogram),
	])?;

	let reduced_gradient = emitter.intermediate(DType::F32, scalar_shape(request)?)?;
	let reduced_hessian = emitter.intermediate(DType::F32, scalar_shape(request)?)?;
	emitter.emit_stage([
		KernelEmission {
			inputs: vec![gradient_histogram],
			outputs: vec![reduced_gradient],
			kind: reduction(
				request,
				ReduceOperator::Sum,
				&[0],
				false,
				ReduceResult::Value,
				tree_lanes,
			)?,
		},
		KernelEmission {
			inputs: vec![hessian_histogram],
			outputs: vec![reduced_hessian],
			kind: reduction(
				request,
				ReduceOperator::Sum,
				&[0],
				false,
				ReduceResult::Value,
				tree_lanes,
			)?,
		},
	])?;
	let mut outputs = match sum_outputs {
		Some((gradient, hessian)) => vec![gradient.id, hessian.id],
		None => Vec::with_capacity(1),
	};
	outputs.push(leaf_value.id);
	emitter.emit(
		vec![reduced_gradient, reduced_hessian],
		outputs,
		PrimitiveKind::Elementwise(Elementwise {
			program: regularized_leaf_program(request.descriptor.id, lambda, expose_sums)?,
		}),
	)
}

fn emit_leaf_reduce(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(
		request,
		&[
			"leaf_indices",
			"gradients",
			"hessians",
			"gradient_base",
			"hessian_base",
		],
		&["leaf_gradients", "leaf_hessians"],
		&["rows", "leaves", "leaf_indices_verified", "bases_zero"],
	)?;
	let leaf_indices = input(request, "leaf_indices")?;
	let gradients = input(request, "gradients")?;
	let hessians = input(request, "hessians")?;
	let gradient_base = input(request, "gradient_base")?;
	let hessian_base = input(request, "hessian_base")?;
	let leaf_gradients = output(request, "leaf_gradients")?;
	let leaf_hessians = output(request, "leaf_hessians")?;
	let rows = prepared_dimension(request, "rows")?;
	let leaves = prepared_dimension(request, "leaves")?;
	require_true(request, "leaf_indices_verified")?;
	require_true(request, "bases_zero")?;
	require_dtype(request, leaf_indices, DType::I32, "leaf_indices")?;
	require_shape(request, leaf_indices, &[rows], "leaf_indices")?;
	for (name, tensor) in [("gradients", gradients), ("hessians", hessians)] {
		require_dtype(request, tensor, DType::F32, name)?;
		require_shape(request, tensor, &[rows], name)?;
	}
	for (name, tensor) in [
		("gradient_base", gradient_base),
		("hessian_base", hessian_base),
		("leaf_gradients", leaf_gradients),
		("leaf_hessians", leaf_hessians),
	] {
		require_dtype(request, tensor, DType::F32, name)?;
		require_shape(request, tensor, &[leaves], name)?;
	}

	let mapped_indices = emitter.intermediate(DType::I32, leaf_indices.shape.clone())?;
	let mapped_gradients = emitter.intermediate(DType::F32, gradients.shape.clone())?;
	let mapped_hessians = emitter.intermediate(DType::F32, hessians.shape.clone())?;
	emitter.emit(
		vec![leaf_indices.id, gradients.id, hessians.id],
		vec![mapped_indices, mapped_gradients, mapped_hessians],
		PrimitiveKind::Elementwise(Elementwise {
			program: histogram_contribution_program(request.descriptor.id)?,
		}),
	)?;
	let conflict = ScatterConflict::Atomic {
		operation: AtomicOperation::Add,
		ordering: AtomicOrdering::SequentiallyConsistent,
	};
	emitter.emit_stage([
		KernelEmission {
			inputs: vec![gradient_base.id, mapped_indices, mapped_gradients],
			outputs: vec![leaf_gradients.id],
			kind: scatter(conflict),
		},
		KernelEmission {
			inputs: vec![hessian_base.id, mapped_indices, mapped_hessians],
			outputs: vec![leaf_hessians.id],
			kind: scatter(conflict),
		},
	])
}

fn emit_histogram_subtraction(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(
		request,
		&[
			"parent_gradients",
			"parent_hessians",
			"parent_counts",
			"child_gradients",
			"child_hessians",
			"child_counts",
		],
		&[
			"remaining_gradients",
			"remaining_hessians",
			"remaining_counts",
		],
		&["elements"],
	)?;
	let parent_gradients = input(request, "parent_gradients")?;
	let parent_hessians = input(request, "parent_hessians")?;
	let parent_counts = input(request, "parent_counts")?;
	let child_gradients = input(request, "child_gradients")?;
	let child_hessians = input(request, "child_hessians")?;
	let child_counts = input(request, "child_counts")?;
	let remaining_gradients = output(request, "remaining_gradients")?;
	let remaining_hessians = output(request, "remaining_hessians")?;
	let remaining_counts = output(request, "remaining_counts")?;
	let elements = prepared_dimension(request, "elements")?;
	for (name, tensor) in [
		("parent_gradients", parent_gradients),
		("parent_hessians", parent_hessians),
		("child_gradients", child_gradients),
		("child_hessians", child_hessians),
		("remaining_gradients", remaining_gradients),
		("remaining_hessians", remaining_hessians),
	] {
		require_dtype(request, tensor, DType::F32, name)?;
		require_shape(request, tensor, &[elements], name)?;
	}
	for (name, tensor) in [
		("parent_counts", parent_counts),
		("child_counts", child_counts),
		("remaining_counts", remaining_counts),
	] {
		require_dtype(request, tensor, DType::I32, name)?;
		require_shape(request, tensor, &[elements], name)?;
	}
	emitter.emit(
		vec![
			parent_gradients.id,
			parent_hessians.id,
			parent_counts.id,
			child_gradients.id,
			child_hessians.id,
			child_counts.id,
		],
		vec![
			remaining_gradients.id,
			remaining_hessians.id,
			remaining_counts.id,
		],
		PrimitiveKind::Elementwise(Elementwise {
			program: histogram_subtraction_program(request.descriptor.id)?,
		}),
	)
}

fn emit_two_bin_split(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(
		request,
		&[
			"bin_indices",
			"gradient_histogram",
			"hessian_histogram",
			"count_histogram",
			"left_count_index",
			"total_index",
			"feature_zero",
		],
		&["best_gain", "best_feature", "best_bin", "best_left_count"],
		&[
			"features",
			"bins",
			"nodes",
			"lambda",
			"minimum_child_weight",
			"bin_indices_identity",
			"left_count_index_zero",
			"total_index_one",
			"feature_zero_is_zero",
			"count_histogram_nonnegative",
			"positive_gain_guaranteed",
			"tree_lanes",
		],
	)?;
	let bin_indices = input(request, "bin_indices")?;
	let gradients = input(request, "gradient_histogram")?;
	let hessians = input(request, "hessian_histogram")?;
	let counts = input(request, "count_histogram")?;
	let left_count_index = input(request, "left_count_index")?;
	let total_index = input(request, "total_index")?;
	let feature_zero = input(request, "feature_zero")?;
	let best_gain = output(request, "best_gain")?;
	let best_feature = output(request, "best_feature")?;
	let best_bin = output(request, "best_bin")?;
	let best_left_count = output(request, "best_left_count")?;
	let features = prepared_dimension(request, "features")?;
	let bins = prepared_dimension(request, "bins")?;
	let nodes = prepared_dimension(request, "nodes")?;
	require_supported(
		request,
		features == 1 && bins == 2 && nodes == 1,
		"the exact split-selection graph currently supports one node, one feature, and two bins",
	)?;
	let lambda = finite_positive(request, "lambda")?;
	let minimum_child_weight = finite_nonnegative(request, "minimum_child_weight")?;
	let tree_lanes = prepared_tree_lanes(request)?;
	for fact in [
		"bin_indices_identity",
		"left_count_index_zero",
		"total_index_one",
		"feature_zero_is_zero",
		"count_histogram_nonnegative",
		"positive_gain_guaranteed",
	] {
		require_true(request, fact)?;
	}
	require_dtype(request, bin_indices, DType::I32, "bin_indices")?;
	require_shape(request, bin_indices, &[2], "bin_indices")?;
	for (name, tensor) in [
		("gradient_histogram", gradients),
		("hessian_histogram", hessians),
	] {
		require_dtype(request, tensor, DType::F32, name)?;
		require_shape(request, tensor, &[2], name)?;
	}
	require_dtype(request, counts, DType::I32, "count_histogram")?;
	require_shape(request, counts, &[2], "count_histogram")?;
	for (name, tensor) in [
		("left_count_index", left_count_index),
		("total_index", total_index),
		("feature_zero", feature_zero),
	] {
		require_dtype(request, tensor, DType::I32, name)?;
		require_shape(request, tensor, &[1], name)?;
	}
	require_scalar(request, best_gain, DType::F32, "best_gain")?;
	require_scalar(request, best_feature, DType::I32, "best_feature")?;
	require_scalar(request, best_bin, DType::I32, "best_bin")?;
	require_scalar(request, best_left_count, DType::I32, "best_left_count")?;

	let copied_gradients = emitter.intermediate(DType::F32, gradients.shape.clone())?;
	let copied_hessians = emitter.intermediate(DType::F32, hessians.shape.clone())?;
	let weighted_histogram = |weights, destination| {
		KernelEmission {
			inputs: vec![bin_indices.id, weights],
			outputs: vec![destination],
			kind: PrimitiveKind::Histogram(Histogram {
				bins: 2,
				weighted: true,
				ordering: AtomicOrdering::SequentiallyConsistent,
			}),
		}
	};
	emitter.emit_stage([
		weighted_histogram(gradients.id, copied_gradients),
		weighted_histogram(hessians.id, copied_hessians),
	])?;

	let prefix_gradients = emitter.intermediate(DType::F32, gradients.shape.clone())?;
	let prefix_hessians = emitter.intermediate(DType::F32, hessians.shape.clone())?;
	let prefix_counts = emitter.intermediate(DType::I32, counts.shape.clone())?;
	emitter.emit_stage([
		KernelEmission {
			inputs: vec![copied_gradients],
			outputs: vec![prefix_gradients],
			kind: inclusive_sum_scan(tree_lanes),
		},
		KernelEmission {
			inputs: vec![copied_hessians],
			outputs: vec![prefix_hessians],
			kind: inclusive_sum_scan(tree_lanes),
		},
		KernelEmission {
			inputs: vec![counts.id],
			outputs: vec![prefix_counts],
			kind: inclusive_sum_scan(tree_lanes),
		},
	])?;

	let total_gradient = emitter.intermediate(DType::F32, scalar_shape(request)?)?;
	let total_hessian = emitter.intermediate(DType::F32, scalar_shape(request)?)?;
	let gains = emitter.intermediate(DType::F32, gradients.shape.clone())?;
	emitter.emit_stage([
		KernelEmission {
			inputs: vec![prefix_gradients, total_index.id],
			outputs: vec![total_gradient],
			kind: gather(),
		},
		KernelEmission {
			inputs: vec![prefix_hessians, total_index.id],
			outputs: vec![total_hessian],
			kind: gather(),
		},
		KernelEmission {
			inputs: vec![
				prefix_gradients,
				prefix_hessians,
				total_gradient,
				total_hessian,
				bin_indices.id,
			],
			outputs: vec![gains],
			kind: PrimitiveKind::Elementwise(Elementwise {
				program: two_bin_gain_program(request.descriptor.id, lambda, minimum_child_weight)?,
			}),
		},
	])?;
	emitter.emit_stage([
		KernelEmission {
			inputs: vec![feature_zero.id],
			outputs: vec![best_feature.id],
			kind: PrimitiveKind::Elementwise(Elementwise {
				program: identity_program(request.descriptor.id, DType::I32)?,
			}),
		},
		KernelEmission {
			inputs: vec![prefix_counts, left_count_index.id],
			outputs: vec![best_left_count.id],
			kind: gather(),
		},
		KernelEmission {
			inputs: vec![gains],
			outputs: vec![best_gain.id, best_bin.id],
			kind: reduction(
				request,
				ReduceOperator::Maximum,
				&[0],
				false,
				ReduceResult::ValueAndIndex,
				tree_lanes,
			)?,
		},
	])
}

fn emit_ordered_target_statistics(
	request: &MaterializationRequest<'_>,
	emitter: &mut Emitter<'_>,
) -> OperationResult<()> {
	require_exact_abi(
		request,
		&["ordering_keys", "targets", "sum_base", "count_base"],
		&["encoded"],
		&[
			"rows",
			"prior",
			"smoothing",
			"one_category",
			"ordering_keys_unique",
			"bases_zero",
			"tree_lanes",
		],
	)?;
	let ordering_keys = input(request, "ordering_keys")?;
	let targets = input(request, "targets")?;
	let sum_base = input(request, "sum_base")?;
	let count_base = input(request, "count_base")?;
	let encoded = output(request, "encoded")?;
	let rows = prepared_dimension(request, "rows")?;
	let prior = finite_parameter(request, "prior")?;
	let smoothing = finite_positive(request, "smoothing")?;
	let tree_lanes = prepared_tree_lanes(request)?;
	for fact in ["one_category", "ordering_keys_unique", "bases_zero"] {
		require_true(request, fact)?;
	}
	require_dtype(request, ordering_keys, DType::I32, "ordering_keys")?;
	require_shape(request, ordering_keys, &[rows], "ordering_keys")?;
	for (name, tensor) in [
		("targets", targets),
		("sum_base", sum_base),
		("count_base", count_base),
		("encoded", encoded),
	] {
		require_dtype(request, tensor, DType::F32, name)?;
		require_shape(request, tensor, &[rows], name)?;
	}

	let sorted_keys = emitter.intermediate(DType::I32, ordering_keys.shape.clone())?;
	let sorted_positions = emitter.intermediate(DType::I32, ordering_keys.shape.clone())?;
	emitter.emit(
		vec![ordering_keys.id],
		vec![sorted_keys, sorted_positions],
		PrimitiveKind::Sort(Sort {
			axis: 0,
			direction: SortDirection::Ascending,
			stable: true,
			emit_indices: true,
		}),
	)?;
	let sorted_targets = emitter.intermediate(DType::F32, targets.shape.clone())?;
	let ones = emitter.intermediate(DType::F32, targets.shape.clone())?;
	let prefix_sums = emitter.intermediate(DType::F32, targets.shape.clone())?;
	let prefix_counts = emitter.intermediate(DType::F32, targets.shape.clone())?;
	emitter.emit_stage([
		KernelEmission {
			inputs: vec![targets.id, sorted_positions],
			outputs: vec![sorted_targets],
			kind: gather(),
		},
		KernelEmission {
			inputs: vec![sorted_targets],
			outputs: vec![ones],
			kind: PrimitiveKind::Elementwise(Elementwise {
				program: constant_one_program(request.descriptor.id)?,
			}),
		},
		KernelEmission {
			inputs: vec![sorted_targets],
			outputs: vec![prefix_sums],
			kind: exclusive_sum_scan(DType::F32, tree_lanes),
		},
		KernelEmission {
			inputs: vec![ones],
			outputs: vec![prefix_counts],
			kind: exclusive_sum_scan(DType::F32, tree_lanes),
		},
	])?;
	let original_sums = emitter.intermediate(DType::F32, targets.shape.clone())?;
	let original_counts = emitter.intermediate(DType::F32, targets.shape.clone())?;
	emitter.emit_stage([
		KernelEmission {
			inputs: vec![sum_base.id, sorted_positions, prefix_sums],
			outputs: vec![original_sums],
			kind: scatter(ScatterConflict::UniqueIndices),
		},
		KernelEmission {
			inputs: vec![count_base.id, sorted_positions, prefix_counts],
			outputs: vec![original_counts],
			kind: scatter(ScatterConflict::UniqueIndices),
		},
		KernelEmission {
			inputs: vec![original_sums, original_counts],
			outputs: vec![encoded.id],
			kind: PrimitiveKind::Elementwise(Elementwise {
				program: ordered_statistic_program(request.descriptor.id, prior, smoothing)?,
			}),
		},
	])
}

fn emit_random_threshold(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(request, &["minimum", "maximum"], &["threshold"], &[
		"seed_low",
		"seed_high",
		"stream",
	])?;
	let minimum = input(request, "minimum")?;
	let maximum = input(request, "maximum")?;
	let threshold = output(request, "threshold")?;
	let key = prepared_random_key(request)?;
	for (name, tensor) in [
		("minimum", minimum),
		("maximum", maximum),
		("threshold", threshold),
	] {
		require_scalar(request, tensor, DType::F32, name)?;
	}
	let uniform = emitter.intermediate(DType::F32, scalar_shape(request)?)?;
	emitter.emit(
		Vec::new(),
		vec![uniform],
		PrimitiveKind::Random(RandomMap {
			distribution: RandomDistribution::UniformF32,
			key,
			philox_rounds: 10,
		}),
	)?;
	emitter.emit(
		vec![minimum.id, maximum.id, uniform],
		vec![threshold.id],
		PrimitiveKind::Elementwise(Elementwise {
			program: threshold_program(request.descriptor.id)?,
		}),
	)
}

fn emit_leaf_prediction(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(
		request,
		&[
			"prediction_base",
			"leaf_values",
			"leaf_indices",
			"destination_indices",
		],
		&["predictions"],
		&[
			"prediction_elements",
			"updates",
			"leaves",
			"learning_rate",
			"leaf_indices_verified",
			"destination_indices_unique",
		],
	)?;
	let prediction_base = input(request, "prediction_base")?;
	let leaf_values = input(request, "leaf_values")?;
	let leaf_indices = input(request, "leaf_indices")?;
	let destination_indices = input(request, "destination_indices")?;
	let predictions = output(request, "predictions")?;
	let prediction_elements = prepared_dimension(request, "prediction_elements")?;
	let updates = prepared_dimension(request, "updates")?;
	let leaves = prepared_dimension(request, "leaves")?;
	let learning_rate = finite_parameter(request, "learning_rate")?;
	require_true(request, "leaf_indices_verified")?;
	require_true(request, "destination_indices_unique")?;
	for (name, tensor) in [
		("prediction_base", prediction_base),
		("predictions", predictions),
	] {
		require_dtype(request, tensor, DType::F32, name)?;
		require_shape(request, tensor, &[prediction_elements], name)?;
	}
	require_dtype(request, leaf_values, DType::F32, "leaf_values")?;
	require_shape(request, leaf_values, &[leaves], "leaf_values")?;
	for (name, tensor) in [
		("leaf_indices", leaf_indices),
		("destination_indices", destination_indices),
	] {
		require_dtype(request, tensor, DType::I32, name)?;
		require_shape(request, tensor, &[updates], name)?;
	}

	let selected_values = emitter.intermediate(DType::F32, shape(request, &[updates])?)?;
	let current_predictions = emitter.intermediate(DType::F32, shape(request, &[updates])?)?;
	let mapped_updates = emitter.intermediate(DType::F32, shape(request, &[updates])?)?;
	emitter.emit_stage([
		KernelEmission {
			inputs: vec![leaf_values.id, leaf_indices.id],
			outputs: vec![selected_values],
			kind: gather(),
		},
		KernelEmission {
			inputs: vec![prediction_base.id, destination_indices.id],
			outputs: vec![current_predictions],
			kind: gather(),
		},
		KernelEmission {
			inputs: vec![current_predictions, selected_values],
			outputs: vec![mapped_updates],
			kind: PrimitiveKind::Elementwise(Elementwise {
				program: leaf_prediction_program(request.descriptor.id, learning_rate)?,
			}),
		},
	])?;
	emitter.emit(
		vec![prediction_base.id, destination_indices.id, mapped_updates],
		vec![predictions.id],
		scatter(ScatterConflict::UniqueIndices),
	)
}

fn emit_write_split(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(
		request,
		&["feature_base", "bin_base", "write_mask"],
		&["split_features", "split_bins"],
		&["slots", "feature", "bin", "write_mask_one_hot"],
	)?;
	let feature_base = input(request, "feature_base")?;
	let bin_base = input(request, "bin_base")?;
	let write_mask = input(request, "write_mask")?;
	let split_features = output(request, "split_features")?;
	let split_bins = output(request, "split_bins")?;
	let slots = prepared_dimension(request, "slots")?;
	let feature = prepared_i32(request, "feature")?;
	let bin = prepared_i32(request, "bin")?;
	require_true(request, "write_mask_one_hot")?;
	for (name, tensor) in [
		("feature_base", feature_base),
		("bin_base", bin_base),
		("write_mask", write_mask),
		("split_features", split_features),
		("split_bins", split_bins),
	] {
		require_dtype(request, tensor, DType::I32, name)?;
		require_shape(request, tensor, &[slots], name)?;
	}
	emitter.emit(
		vec![feature_base.id, bin_base.id, write_mask.id],
		vec![split_features.id, split_bins.id],
		PrimitiveKind::Elementwise(Elementwise {
			program: write_split_program(request.descriptor.id, feature, bin)?,
		}),
	)
}

fn gather() -> PrimitiveKind {
	PrimitiveKind::Gather(Gather {
		axis: 0,
		bounds: IndexBounds::Reject,
	})
}

fn scatter(conflict: ScatterConflict) -> PrimitiveKind {
	PrimitiveKind::Scatter(Scatter {
		axis: 0,
		bounds: IndexBounds::Reject,
		conflict,
	})
}

fn reduction(
	request: &MaterializationRequest<'_>,
	operator: ReduceOperator,
	axes: &[usize],
	keep_dimensions: bool,
	result: ReduceResult,
	tree_lanes: u32,
) -> OperationResult<PrimitiveKind> {
	let axes =
		AxisSet::new(axes.to_vec()).map_err(|error| language_error(request.descriptor.id, error.to_string()))?;
	Ok(PrimitiveKind::Reduce(Reduce {
		operator,
		axes,
		keep_dimensions,
		result,
		tree_lanes,
	}))
}

fn inclusive_sum_scan(tree_lanes: u32) -> PrimitiveKind {
	PrimitiveKind::Scan(Scan {
		operator: ReduceOperator::Sum,
		axis: 0,
		mode: ScanMode::Inclusive,
		reverse: false,
		tree_lanes,
	})
}

fn exclusive_sum_scan(dtype: DType, tree_lanes: u32) -> PrimitiveKind {
	let identity = match dtype {
		DType::F32 => recipe_core::ScalarLiteral::F32Bits(0.0_f32.to_bits()),
		DType::I32 => recipe_core::ScalarLiteral::I32(0),
	};
	PrimitiveKind::Scan(Scan {
		operator: ReduceOperator::Sum,
		axis: 0,
		mode: ScanMode::Exclusive { identity },
		reverse: false,
		tree_lanes,
	})
}

fn shape(request: &MaterializationRequest<'_>, extents: &[u64]) -> OperationResult<Shape> {
	Shape::new(extents.to_vec()).map_err(|error| language_error(request.descriptor.id, error.to_string()))
}

fn scalar_shape(request: &MaterializationRequest<'_>) -> OperationResult<Shape> { shape(request, &[1]) }

fn prepared_dimension(request: &MaterializationRequest<'_>, name: &str) -> OperationResult<u64> {
	let value = prepared_u64(request.descriptor.id, request.parameters, name)?;
	match value > 0 && value <= MAX_I32_INDEX {
		true => Ok(value),
		false => {
			Err(unsupported(
				request,
				format!("{name} must be in 1..={MAX_I32_INDEX}, got {value}"),
			))
		}
	}
}

fn prepared_i32(request: &MaterializationRequest<'_>, name: &str) -> OperationResult<i32> {
	let value = prepared_u64(request.descriptor.id, request.parameters, name)?;
	i32::try_from(value).map_err(|conversion_error| request_error(request, conversion_error.to_string()))
}

fn prepared_random_key(request: &MaterializationRequest<'_>) -> OperationResult<RandomKey> {
	Ok(RandomKey {
		seed_low: prepared_u64(request.descriptor.id, request.parameters, "seed_low")?,
		seed_high: prepared_u64(request.descriptor.id, request.parameters, "seed_high")?,
		stream: prepared_u64(request.descriptor.id, request.parameters, "stream")?,
	})
}

fn finite_parameter(request: &MaterializationRequest<'_>, name: &str) -> OperationResult<f32> {
	let value = prepared_f32(request, name)?;
	match value.is_finite() {
		true => Ok(value),
		false => Err(request_error(request, format!("{name} must be finite"))),
	}
}

fn finite_nonnegative(request: &MaterializationRequest<'_>, name: &str) -> OperationResult<f32> {
	let value = finite_parameter(request, name)?;
	match value >= 0.0 {
		true => Ok(value),
		false => {
			Err(request_error(
				request,
				format!("{name} must be nonnegative"),
			))
		}
	}
}

fn finite_positive(request: &MaterializationRequest<'_>, name: &str) -> OperationResult<f32> {
	let value = finite_parameter(request, name)?;
	match value > 0.0 {
		true => Ok(value),
		false => Err(request_error(request, format!("{name} must be positive"))),
	}
}

fn require_scalar(
	request: &MaterializationRequest<'_>,
	tensor: &Tensor,
	dtype: DType,
	name: &str,
) -> OperationResult<()> {
	require_dtype(request, tensor, dtype, name)?;
	require_shape(request, tensor, &[1], name)
}

fn require_supported(
	request: &MaterializationRequest<'_>,
	condition: bool,
	detail: &'static str,
) -> OperationResult<()> {
	match condition {
		true => Ok(()),
		false => Err(unsupported(request, detail)),
	}
}

fn unsupported(request: &MaterializationRequest<'_>, detail: impl Into<String>) -> crate::OperationError {
	operation_error(
		request.descriptor.id,
		OperationErrorKind::UnsupportedConcreteShape,
		detail,
	)
}

fn bool_mask(
	operation: OperationId,
	builder: &mut recipe_language::ScalarProgramBuilder,
	mask: recipe_language::ScalarExpression,
) -> OperationResult<recipe_language::ScalarExpression> {
	let zero = builder
		.i32(0)
		.map_err(|error| language_error(operation, error.to_string()))?;
	let one = builder
		.i32(1)
		.map_err(|error| language_error(operation, error.to_string()))?;
	let is_zero = scalar_binary(operation, builder, ScalarOpcode::Equal, mask, zero)?;
	let is_one = scalar_binary(operation, builder, ScalarOpcode::Equal, mask, one)?;
	let valid = scalar_binary(operation, builder, ScalarOpcode::BitOr, is_zero, is_one)?;
	scalar_unary(operation, builder, ScalarOpcode::Require, valid)?;
	Ok(mask)
}

fn split_index_program(operation: OperationId, bins: u64) -> OperationResult<recipe_core::ScalarProgram> {
	let bins = i32::try_from(bins).map_err(|conversion_error| {
		operation_error(
			operation,
			OperationErrorKind::InvalidMaterializationRequest,
			conversion_error.to_string(),
		)
	})?;
	let mut builder = scalar_builder(operation)?;
	let index = scalar_input(operation, &mut builder, DType::I32)?;
	let divisor = builder
		.i32(bins)
		.map_err(|error| language_error(operation, error.to_string()))?;
	let feature = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Divide,
		index,
		divisor,
	)?;
	let bin = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Remainder,
		index,
		divisor,
	)?;
	scalar_finish(operation, builder, &[feature, bin])
}

fn gradient_hessian_program(operation: OperationId, class_index: i32) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let probability = scalar_input(operation, &mut builder, DType::F32)?;
	let target = scalar_input(operation, &mut builder, DType::I32)?;
	let weight = scalar_input(operation, &mut builder, DType::F32)?;
	let mask = scalar_input(operation, &mut builder, DType::I32)?;
	let mask = bool_mask(operation, &mut builder, mask)?;
	let class_index = builder
		.i32(class_index)
		.map_err(|error| language_error(operation, error.to_string()))?;
	let target_matches = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Equal,
		target,
		class_index,
	)?;
	let target_matches = scalar_unary(
		operation,
		&mut builder,
		ScalarOpcode::ConvertI32ToF32,
		target_matches,
	)?;
	let mask = scalar_unary(operation, &mut builder, ScalarOpcode::ConvertI32ToF32, mask)?;
	let residual = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Subtract,
		probability,
		target_matches,
	)?;
	let weighted_residual = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		residual,
		weight,
	)?;
	let gradient = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		weighted_residual,
		mask,
	)?;
	let one = scalar_f32(operation, &mut builder, 1.0)?;
	let complement = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Subtract,
		one,
		probability,
	)?;
	let variance = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		probability,
		complement,
	)?;
	let weighted_variance = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		variance,
		weight,
	)?;
	let minimum = scalar_f32(operation, &mut builder, 0.001)?;
	let maximum = scalar_f32(operation, &mut builder, 1_000_000.0)?;
	let floored = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Maximum,
		weighted_variance,
		minimum,
	)?;
	let clamped = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Minimum,
		floored,
		maximum,
	)?;
	let hessian = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		clamped,
		mask,
	)?;
	scalar_finish(operation, builder, &[gradient, hessian])
}

fn histogram_contribution_program(operation: OperationId) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let bin = scalar_input(operation, &mut builder, DType::I32)?;
	let gradient = scalar_input(operation, &mut builder, DType::F32)?;
	let hessian = scalar_input(operation, &mut builder, DType::F32)?;
	scalar_finish(operation, builder, &[bin, gradient, hessian])
}

fn regularized_leaf_program(
	operation: OperationId,
	lambda: f32,
	expose_sums: bool,
) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let gradient = scalar_input(operation, &mut builder, DType::F32)?;
	let hessian = scalar_input(operation, &mut builder, DType::F32)?;
	let zero = scalar_f32(operation, &mut builder, 0.0)?;
	let nonnegative_hessian = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::GreaterThanOrEqual,
		hessian,
		zero,
	)?;
	scalar_unary(
		operation,
		&mut builder,
		ScalarOpcode::Require,
		nonnegative_hessian,
	)?;
	let lambda = scalar_f32(operation, &mut builder, lambda)?;
	let denominator = scalar_binary(operation, &mut builder, ScalarOpcode::Add, hessian, lambda)?;
	let positive_denominator = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::GreaterThan,
		denominator,
		zero,
	)?;
	scalar_unary(
		operation,
		&mut builder,
		ScalarOpcode::Require,
		positive_denominator,
	)?;
	let negative_gradient = scalar_unary(operation, &mut builder, ScalarOpcode::Negate, gradient)?;
	let value = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Divide,
		negative_gradient,
		denominator,
	)?;
	let outputs = match expose_sums {
		true => vec![gradient, hessian, value],
		false => vec![value],
	};
	scalar_finish(operation, builder, &outputs)
}

fn histogram_subtraction_program(operation: OperationId) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let parent_gradient = scalar_input(operation, &mut builder, DType::F32)?;
	let parent_hessian = scalar_input(operation, &mut builder, DType::F32)?;
	let parent_count = scalar_input(operation, &mut builder, DType::I32)?;
	let child_gradient = scalar_input(operation, &mut builder, DType::F32)?;
	let child_hessian = scalar_input(operation, &mut builder, DType::F32)?;
	let child_count = scalar_input(operation, &mut builder, DType::I32)?;
	let gradient = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Subtract,
		parent_gradient,
		child_gradient,
	)?;
	let hessian = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Subtract,
		parent_hessian,
		child_hessian,
	)?;
	let count = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Subtract,
		parent_count,
		child_count,
	)?;
	scalar_finish(operation, builder, &[gradient, hessian, count])
}

fn two_bin_gain_program(
	operation: OperationId,
	lambda: f32,
	minimum_child_weight: f32,
) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let left_gradient = scalar_input(operation, &mut builder, DType::F32)?;
	let left_hessian = scalar_input(operation, &mut builder, DType::F32)?;
	let total_gradient = scalar_input(operation, &mut builder, DType::F32)?;
	let total_hessian = scalar_input(operation, &mut builder, DType::F32)?;
	let bin = scalar_input(operation, &mut builder, DType::I32)?;
	for value in [left_gradient, left_hessian, total_gradient, total_hessian] {
		let finite = scalar_unary(operation, &mut builder, ScalarOpcode::IsFinite, value)?;
		scalar_unary(operation, &mut builder, ScalarOpcode::Require, finite)?;
	}
	let zero_f32 = scalar_f32(operation, &mut builder, 0.0)?;
	let total_hessian_nonnegative = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::GreaterThanOrEqual,
		total_hessian,
		zero_f32,
	)?;
	scalar_unary(
		operation,
		&mut builder,
		ScalarOpcode::Require,
		total_hessian_nonnegative,
	)?;
	let zero_i32 = builder
		.i32(0)
		.map_err(|error| language_error(operation, error.to_string()))?;
	let one_i32 = builder
		.i32(1)
		.map_err(|error| language_error(operation, error.to_string()))?;
	let is_left_candidate = scalar_binary(operation, &mut builder, ScalarOpcode::Equal, bin, zero_i32)?;
	let is_terminal_bin = scalar_binary(operation, &mut builder, ScalarOpcode::Equal, bin, one_i32)?;
	let valid_bin = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::BitOr,
		is_left_candidate,
		is_terminal_bin,
	)?;
	scalar_unary(operation, &mut builder, ScalarOpcode::Require, valid_bin)?;
	let right_gradient = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Subtract,
		total_gradient,
		left_gradient,
	)?;
	let right_hessian = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Subtract,
		total_hessian,
		left_hessian,
	)?;
	let minimum_child_weight = scalar_f32(operation, &mut builder, minimum_child_weight)?;
	let left_legal = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::GreaterThanOrEqual,
		left_hessian,
		minimum_child_weight,
	)?;
	let right_legal = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::GreaterThanOrEqual,
		right_hessian,
		minimum_child_weight,
	)?;
	let legal_split = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::BitAnd,
		left_legal,
		right_legal,
	)?;
	let valid_candidate = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::BitOr,
		is_terminal_bin,
		legal_split,
	)?;
	scalar_unary(
		operation,
		&mut builder,
		ScalarOpcode::Require,
		valid_candidate,
	)?;
	let lambda = scalar_f32(operation, &mut builder, lambda)?;
	let left_denominator = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Add,
		left_hessian,
		lambda,
	)?;
	let right_denominator = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Add,
		right_hessian,
		lambda,
	)?;
	let total_denominator = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Add,
		total_hessian,
		lambda,
	)?;
	let left_square = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		left_gradient,
		left_gradient,
	)?;
	let right_square = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		right_gradient,
		right_gradient,
	)?;
	let total_square = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		total_gradient,
		total_gradient,
	)?;
	let left_score = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Divide,
		left_square,
		left_denominator,
	)?;
	let right_score = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Divide,
		right_square,
		right_denominator,
	)?;
	let parent_score = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Divide,
		total_square,
		total_denominator,
	)?;
	let child_score = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Add,
		left_score,
		right_score,
	)?;
	let gain = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Subtract,
		child_score,
		parent_score,
	)?;
	let terminal_sentinel = scalar_f32(operation, &mut builder, -f32::MAX)?;
	let selected = scalar_ternary(
		operation,
		&mut builder,
		ScalarOpcode::Select,
		is_left_candidate,
		gain,
		terminal_sentinel,
	)?;
	scalar_finish(operation, builder, &[selected])
}

fn constant_one_program(operation: OperationId) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let consumed = scalar_input(operation, &mut builder, DType::F32)?;
	let one = scalar_f32(operation, &mut builder, 1.0)?;
	let always = builder
		.i32(1)
		.map_err(|error| language_error(operation, error.to_string()))?;
	let result = scalar_ternary(
		operation,
		&mut builder,
		ScalarOpcode::Select,
		always,
		one,
		consumed,
	)?;
	scalar_finish(operation, builder, &[result])
}

fn ordered_statistic_program(
	operation: OperationId,
	prior: f32,
	smoothing: f32,
) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let prefix_sum = scalar_input(operation, &mut builder, DType::F32)?;
	let prefix_count = scalar_input(operation, &mut builder, DType::F32)?;
	let prior = scalar_f32(operation, &mut builder, prior)?;
	let smoothing = scalar_f32(operation, &mut builder, smoothing)?;
	let smoothed_prior = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		prior,
		smoothing,
	)?;
	let numerator = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Add,
		prefix_sum,
		smoothed_prior,
	)?;
	let denominator = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Add,
		prefix_count,
		smoothing,
	)?;
	let statistic = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Divide,
		numerator,
		denominator,
	)?;
	scalar_finish(operation, builder, &[statistic])
}

fn threshold_program(operation: OperationId) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let minimum = scalar_input(operation, &mut builder, DType::F32)?;
	let maximum = scalar_input(operation, &mut builder, DType::F32)?;
	let uniform = scalar_input(operation, &mut builder, DType::F32)?;
	for value in [minimum, maximum, uniform] {
		let finite = scalar_unary(operation, &mut builder, ScalarOpcode::IsFinite, value)?;
		scalar_unary(operation, &mut builder, ScalarOpcode::Require, finite)?;
	}
	let ordered = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::LessThanOrEqual,
		minimum,
		maximum,
	)?;
	scalar_unary(operation, &mut builder, ScalarOpcode::Require, ordered)?;
	let width = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Subtract,
		maximum,
		minimum,
	)?;
	let offset = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		uniform,
		width,
	)?;
	let threshold = scalar_binary(operation, &mut builder, ScalarOpcode::Add, minimum, offset)?;
	scalar_finish(operation, builder, &[threshold])
}

fn leaf_prediction_program(operation: OperationId, learning_rate: f32) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let current = scalar_input(operation, &mut builder, DType::F32)?;
	let leaf_value = scalar_input(operation, &mut builder, DType::F32)?;
	let learning_rate = scalar_f32(operation, &mut builder, learning_rate)?;
	let update = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		leaf_value,
		learning_rate,
	)?;
	let prediction = scalar_binary(operation, &mut builder, ScalarOpcode::Add, current, update)?;
	scalar_finish(operation, builder, &[prediction])
}

fn write_split_program(operation: OperationId, feature: i32, bin: i32) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let feature_base = scalar_input(operation, &mut builder, DType::I32)?;
	let bin_base = scalar_input(operation, &mut builder, DType::I32)?;
	let mask = scalar_input(operation, &mut builder, DType::I32)?;
	let mask = bool_mask(operation, &mut builder, mask)?;
	let feature = builder
		.i32(feature)
		.map_err(|error| language_error(operation, error.to_string()))?;
	let bin = builder
		.i32(bin)
		.map_err(|error| language_error(operation, error.to_string()))?;
	let selected_feature = scalar_ternary(
		operation,
		&mut builder,
		ScalarOpcode::Select,
		mask,
		feature,
		feature_base,
	)?;
	let selected_bin = scalar_ternary(
		operation,
		&mut builder,
		ScalarOpcode::Select,
		mask,
		bin,
		bin_base,
	)?;
	scalar_finish(operation, builder, &[selected_feature, selected_bin])
}
