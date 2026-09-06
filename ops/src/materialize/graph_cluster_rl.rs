use recipe_core::{DType, ScalarLiteral, ScalarOpcode};
use recipe_language::{AtomicOperation, AtomicOrdering, AxisSet, Contraction, Elementwise, Gather, Histogram, IndexBounds, PrimitiveKind, Reduce, ReduceOperator, ReduceResult, Scan, ScanMode, Scatter, ScatterConflict, Shape, Sort, SortDirection};
use recipe_math::MathFunction;

use super::{Emitter, FamilyDispatch, KernelEmission, MAX_I32_INDEX, MaterializationRequest, identity_program, input, language_error, operation_error, output, prepared_f32, prepared_tree_lanes, prepared_u64, request_error, require_dtype, require_exact_abi, require_shape, require_true, scalar_binary, scalar_builder, scalar_f32, scalar_finish, scalar_input, scalar_ternary, scalar_unary};
use crate::{OperationDescriptor, OperationErrorKind, OperationId, OperationResult};

const OPERATIONS: &[(&str, &str)] = &[
	("gpu_boruvka_mst", "gpu-core/src/cluster.rs:395"),
	("gpu_categorical_logprob", "gpu-core/src/rl.rs:131"),
	("gpu_centroid_update", "gpu-core/src/kernels.rs:4685"),
	("gpu_core_distance", "gpu-core/src/cluster.rs:476"),
	("gpu_csr_spmm", "gpu-core/src/graph.rs:82"),
	("gpu_csr_spmv", "gpu-core/src/graph.rs:56"),
	("gpu_degree", "gpu-core/src/graph.rs:158"),
	("gpu_discounted_returns", "gpu-core/src/rl.rs:57"),
	("gpu_fixed_radius_neighbors", "gpu-core/src/cluster.rs:179"),
	("gpu_gae", "gpu-core/src/rl.rs:79"),
	("gpu_gaussian_logprob", "gpu-core/src/rl.rs:156"),
	("gpu_gcn_norm", "gpu-core/src/graph.rs:180"),
	("gpu_neighbor_aggregate", "gpu-core/src/graph.rs:111"),
	("gpu_pairwise_l2", "gpu-core/src/kernels.rs:5239"),
	("gpu_td_targets", "gpu-core/src/rl.rs:105"),
	("gpu_union_find_cc", "gpu-core/src/cluster.rs:251"),
];

pub(super) fn supports(descriptor: OperationDescriptor) -> bool { OPERATIONS.contains(&(descriptor.symbol, descriptor.source)) }

pub(super) fn dispatch(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> FamilyDispatch {
	match supports(request.descriptor) {
		false => FamilyDispatch::NotOwned,
		true => {
			let result = match request.descriptor.symbol {
				"gpu_boruvka_mst" => emit_boruvka(request, emitter),
				"gpu_categorical_logprob" => emit_categorical_log_probability(request, emitter),
				"gpu_centroid_update" => emit_centroid_update(request, emitter),
				"gpu_core_distance" => emit_core_distance(request, emitter),
				"gpu_csr_spmm" => emit_csr_sparse_matrix(request, emitter),
				"gpu_csr_spmv" => emit_csr_sparse_vector(request, emitter),
				"gpu_degree" => emit_degree(request, emitter),
				"gpu_discounted_returns" => emit_discounted_returns(request, emitter),
				"gpu_fixed_radius_neighbors" => emit_fixed_radius_singleton(request, emitter),
				"gpu_gae" => emit_generalized_advantage(request, emitter),
				"gpu_gaussian_logprob" => emit_gaussian_log_probability(request, emitter),
				"gpu_gcn_norm" => emit_gcn_normalization(request, emitter),
				"gpu_neighbor_aggregate" => emit_neighbor_aggregate(request, emitter),
				"gpu_pairwise_l2" => emit_pairwise_l2(request, emitter),
				"gpu_td_targets" => emit_temporal_difference_targets(request, emitter),
				"gpu_union_find_cc" => emit_union_find_two_nodes(request, emitter),
				symbol => {
					Err(operation_error(
						request.descriptor.id,
						OperationErrorKind::GraphMaterializationFailed,
						format!("graph/clustering/RL dispatch is incomplete for {symbol}"),
					))
				}
			};
			FamilyDispatch::Owned(result)
		}
	}
}

fn emit_degree(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(request, &["edge_destinations"], &["degrees"], &[
		"nodes",
		"edges",
		"endpoint_indices_verified",
	])?;
	let edge_destinations = input(request, "edge_destinations")?;
	let degrees = output(request, "degrees")?;
	let nodes = prepared_dimension(request, "nodes")?;
	let edges = prepared_count(request, "edges")?;
	require_true(request, "endpoint_indices_verified")?;
	require_dtype(request, edge_destinations, DType::I32, "edge_destinations")?;
	require_dtype(request, degrees, DType::I32, "degrees")?;
	require_shape(request, edge_destinations, &[edges], "edge_destinations")?;
	require_shape(request, degrees, &[nodes], "degrees")?;

	let mapped = emitter.intermediate(DType::I32, edge_destinations.shape.clone())?;
	emitter.emit(
		vec![edge_destinations.id],
		vec![mapped],
		PrimitiveKind::Elementwise(Elementwise {
			program: identity_program(request.descriptor.id, DType::I32)?,
		}),
	)?;
	let bins = u32::try_from(nodes).map_err(|conversion_error| request_error(request, conversion_error.to_string()))?;
	emitter.emit(
		vec![mapped],
		vec![degrees.id],
		PrimitiveKind::Histogram(Histogram {
			bins,
			weighted: false,
			ordering: AtomicOrdering::Relaxed,
		}),
	)
}

fn emit_temporal_difference_targets(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(
		request,
		&["rewards", "values", "next_value_indices", "done_mask"],
		&["targets"],
		&["elements", "gamma", "next_value_indices_verified"],
	)?;
	let rewards = input(request, "rewards")?;
	let values = input(request, "values")?;
	let indices = input(request, "next_value_indices")?;
	let done_mask = input(request, "done_mask")?;
	let targets = output(request, "targets")?;
	let elements = prepared_dimension(request, "elements")?;
	let gamma = finite_parameter(request, "gamma")?;
	require_true(request, "next_value_indices_verified")?;
	for (name, tensor) in [
		("rewards", rewards),
		("values", values),
		("targets", targets),
	] {
		require_dtype(request, tensor, DType::F32, name)?;
		require_shape(request, tensor, &[elements], name)?;
	}
	for (name, tensor) in [("next_value_indices", indices), ("done_mask", done_mask)] {
		require_dtype(request, tensor, DType::I32, name)?;
		require_shape(request, tensor, &[elements], name)?;
	}

	let next_values = emitter.intermediate(DType::F32, values.shape.clone())?;
	emitter.emit(vec![values.id, indices.id], vec![next_values], gather())?;
	emitter.emit(
		vec![rewards.id, next_values, done_mask.id],
		vec![targets.id],
		PrimitiveKind::Elementwise(Elementwise {
			program: temporal_difference_program(request.descriptor.id, gamma)?,
		}),
	)
}

fn emit_gcn_normalization(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(
		request,
		&["features", "degrees", "degree_indices"],
		&["normalized"],
		&["nodes", "features_per_node", "degree_indices_verified"],
	)?;
	let features = input(request, "features")?;
	let degrees = input(request, "degrees")?;
	let indices = input(request, "degree_indices")?;
	let normalized = output(request, "normalized")?;
	let nodes = prepared_dimension(request, "nodes")?;
	let feature_count = prepared_dimension(request, "features_per_node")?;
	let elements = checked_product(request, &[nodes, feature_count], "GCN feature count")?;
	require_true(request, "degree_indices_verified")?;
	for (name, tensor, shape) in [
		("features", features, &[elements][..]),
		("degrees", degrees, &[nodes][..]),
		("normalized", normalized, &[elements][..]),
	] {
		require_dtype(request, tensor, DType::F32, name)?;
		require_shape(request, tensor, shape, name)?;
	}
	require_dtype(request, indices, DType::I32, "degree_indices")?;
	require_shape(request, indices, &[elements], "degree_indices")?;

	let gathered_degrees = emitter.intermediate(DType::F32, features.shape.clone())?;
	emitter.emit(
		vec![degrees.id, indices.id],
		vec![gathered_degrees],
		gather(),
	)?;
	emitter.emit(
		vec![features.id, gathered_degrees],
		vec![normalized.id],
		PrimitiveKind::Elementwise(Elementwise {
			program: gcn_normalization_program(request.descriptor.id)?,
		}),
	)
}

fn emit_discounted_returns(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(request, &["rewards"], &["returns"], &[
		"length",
		"gamma",
		"tree_lanes",
	])?;
	let rewards = input(request, "rewards")?;
	let returns = output(request, "returns")?;
	let length = prepared_dimension(request, "length")?;
	let gamma = finite_parameter(request, "gamma")?;
	require_supported(
		request,
		gamma == 1.0,
		"the fixed primitive scan exactly represents discounted returns only when gamma is 1",
	)?;
	let tree_lanes = prepared_tree_lanes(request)?;
	for (name, tensor) in [("rewards", rewards), ("returns", returns)] {
		require_dtype(request, tensor, DType::F32, name)?;
		require_shape(request, tensor, &[length], name)?;
	}

	let mapped_rewards = emitter.intermediate(DType::F32, rewards.shape.clone())?;
	emitter.emit(
		vec![rewards.id],
		vec![mapped_rewards],
		PrimitiveKind::Elementwise(Elementwise {
			program: identity_program(request.descriptor.id, DType::F32)?,
		}),
	)?;
	emitter.emit(
		vec![mapped_rewards],
		vec![returns.id],
		PrimitiveKind::Scan(Scan {
			operator: ReduceOperator::Sum,
			axis: 0,
			mode: ScanMode::Inclusive,
			reverse: true,
			tree_lanes,
		}),
	)
}

fn emit_generalized_advantage(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(
		request,
		&["rewards", "values", "next_value_indices", "has_next"],
		&["advantages"],
		&[
			"length",
			"gamma",
			"lambda",
			"tree_lanes",
			"next_value_indices_verified",
		],
	)?;
	let rewards = input(request, "rewards")?;
	let values = input(request, "values")?;
	let indices = input(request, "next_value_indices")?;
	let has_next = input(request, "has_next")?;
	let advantages = output(request, "advantages")?;
	let length = prepared_dimension(request, "length")?;
	let gamma = finite_parameter(request, "gamma")?;
	let lambda = finite_parameter(request, "lambda")?;
	require_supported(
		request,
		gamma * lambda == 1.0,
		"the fixed primitive sum scan exactly represents GAE only when gamma * lambda is 1",
	)?;
	let tree_lanes = prepared_tree_lanes(request)?;
	require_true(request, "next_value_indices_verified")?;
	for (name, tensor) in [
		("rewards", rewards),
		("values", values),
		("advantages", advantages),
	] {
		require_dtype(request, tensor, DType::F32, name)?;
		require_shape(request, tensor, &[length], name)?;
	}
	for (name, tensor) in [("next_value_indices", indices), ("has_next", has_next)] {
		require_dtype(request, tensor, DType::I32, name)?;
		require_shape(request, tensor, &[length], name)?;
	}

	let next_values = emitter.intermediate(DType::F32, values.shape.clone())?;
	let deltas = emitter.intermediate(DType::F32, values.shape.clone())?;
	emitter.emit_stage([
		KernelEmission {
			inputs: vec![values.id, indices.id],
			outputs: vec![next_values],
			kind: gather(),
		},
		KernelEmission {
			inputs: vec![rewards.id, values.id, next_values, has_next.id],
			outputs: vec![deltas],
			kind: PrimitiveKind::Elementwise(Elementwise {
				program: advantage_delta_program(request.descriptor.id, gamma)?,
			}),
		},
	])?;
	emitter.emit(
		vec![deltas],
		vec![advantages.id],
		PrimitiveKind::Scan(Scan {
			operator: ReduceOperator::Sum,
			axis: 0,
			mode: ScanMode::Inclusive,
			reverse: true,
			tree_lanes,
		}),
	)
}

fn emit_gaussian_log_probability(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(
		request,
		&["means", "log_standard_deviations", "actions"],
		&["log_probabilities"],
		&["rows", "dimensions", "tree_lanes"],
	)?;
	let means = input(request, "means")?;
	let log_standard_deviations = input(request, "log_standard_deviations")?;
	let actions = input(request, "actions")?;
	let log_probabilities = output(request, "log_probabilities")?;
	let rows = prepared_dimension(request, "rows")?;
	let dimensions = prepared_dimension(request, "dimensions")?;
	let tree_lanes = prepared_tree_lanes(request)?;
	for (name, tensor) in [
		("means", means),
		("log_standard_deviations", log_standard_deviations),
		("actions", actions),
	] {
		require_dtype(request, tensor, DType::F32, name)?;
		require_shape(request, tensor, &[rows, dimensions], name)?;
	}
	require_dtype(request, log_probabilities, DType::F32, "log_probabilities")?;
	require_shape(request, log_probabilities, &[rows], "log_probabilities")?;

	let standard_deviations = emitter.intermediate(DType::F32, means.shape.clone())?;
	let terms = emitter.intermediate(DType::F32, means.shape.clone())?;
	let exponential = recipe_core::ScalarProgram::try_from(MathFunction::Exp).map_err(|error| language_error(request.descriptor.id, error.to_string()))?;
	emitter.emit_stage([
		KernelEmission {
			inputs: vec![log_standard_deviations.id],
			outputs: vec![standard_deviations],
			kind: PrimitiveKind::Elementwise(Elementwise {
				program: exponential,
			}),
		},
		KernelEmission {
			inputs: vec![
				means.id,
				log_standard_deviations.id,
				actions.id,
				standard_deviations,
			],
			outputs: vec![terms],
			kind: PrimitiveKind::Elementwise(Elementwise {
				program: gaussian_term_program(request.descriptor.id)?,
			}),
		},
	])?;
	emitter.emit(
		vec![terms],
		vec![log_probabilities.id],
		reduction(
			request.descriptor.id,
			ReduceOperator::Sum,
			&[1],
			false,
			ReduceResult::Value,
			tree_lanes,
		)?,
	)
}

fn emit_categorical_log_probability(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(
		request,
		&["logits", "flat_logits", "actions", "row_bases"],
		&["log_probabilities"],
		&[
			"rows",
			"action_count",
			"tree_lanes",
			"logit_views_identical",
			"row_bases_verified",
		],
	)?;
	let logits = input(request, "logits")?;
	let flat_logits = input(request, "flat_logits")?;
	let actions = input(request, "actions")?;
	let row_bases = input(request, "row_bases")?;
	let log_probabilities = output(request, "log_probabilities")?;
	let rows = prepared_dimension(request, "rows")?;
	let action_count = prepared_dimension(request, "action_count")?;
	let elements = checked_product(request, &[rows, action_count], "categorical logit count")?;
	let tree_lanes = prepared_tree_lanes(request)?;
	require_true(request, "logit_views_identical")?;
	require_true(request, "row_bases_verified")?;
	require_dtype(request, logits, DType::F32, "logits")?;
	require_shape(request, logits, &[rows, action_count], "logits")?;
	require_dtype(request, flat_logits, DType::F32, "flat_logits")?;
	require_shape(request, flat_logits, &[elements], "flat_logits")?;
	for (name, tensor) in [("actions", actions), ("row_bases", row_bases)] {
		require_dtype(request, tensor, DType::I32, name)?;
		require_shape(request, tensor, &[rows], name)?;
	}
	require_dtype(request, log_probabilities, DType::F32, "log_probabilities")?;
	require_shape(request, log_probabilities, &[rows], "log_probabilities")?;

	let row_shape = shape(request, &[rows])?;
	let kept_row_shape = shape(request, &[rows, 1])?;
	let row_maxima_kept = emitter.intermediate(DType::F32, kept_row_shape)?;
	let row_maxima = emitter.intermediate(DType::F32, row_shape.clone())?;
	let shifted = emitter.intermediate(DType::F32, logits.shape.clone())?;
	let exponentials = emitter.intermediate(DType::F32, logits.shape.clone())?;
	let row_sums = emitter.intermediate(DType::F32, row_shape.clone())?;
	let selected_indices = emitter.intermediate(DType::I32, row_shape.clone())?;
	let selected_logits = emitter.intermediate(DType::F32, row_shape.clone())?;
	let exponential = recipe_core::ScalarProgram::try_from(MathFunction::Exp).map_err(|error| language_error(request.descriptor.id, error.to_string()))?;
	emitter.emit_stage([
		KernelEmission {
			inputs: vec![logits.id],
			outputs: vec![row_maxima_kept],
			kind: reduction(
				request.descriptor.id,
				ReduceOperator::Maximum,
				&[1],
				true,
				ReduceResult::Value,
				tree_lanes,
			)?,
		},
		KernelEmission {
			inputs: vec![logits.id],
			outputs: vec![row_maxima],
			kind: reduction(
				request.descriptor.id,
				ReduceOperator::Maximum,
				&[1],
				false,
				ReduceResult::Value,
				tree_lanes,
			)?,
		},
		KernelEmission {
			inputs: vec![logits.id, row_maxima_kept],
			outputs: vec![shifted],
			kind: PrimitiveKind::Elementwise(Elementwise {
				program: subtract_program(request.descriptor.id)?,
			}),
		},
		KernelEmission {
			inputs: vec![shifted],
			outputs: vec![exponentials],
			kind: PrimitiveKind::Elementwise(Elementwise {
				program: exponential,
			}),
		},
		KernelEmission {
			inputs: vec![exponentials],
			outputs: vec![row_sums],
			kind: reduction(
				request.descriptor.id,
				ReduceOperator::Sum,
				&[1],
				false,
				ReduceResult::Value,
				tree_lanes,
			)?,
		},
		KernelEmission {
			inputs: vec![row_bases.id, actions.id],
			outputs: vec![selected_indices],
			kind: PrimitiveKind::Elementwise(Elementwise {
				program: checked_flat_action_program(request.descriptor.id, action_count)?,
			}),
		},
		KernelEmission {
			inputs: vec![flat_logits.id, selected_indices],
			outputs: vec![selected_logits],
			kind: gather(),
		},
	])?;

	let log_sums = emitter.intermediate(DType::F32, row_shape)?;
	let logarithm = recipe_core::ScalarProgram::try_from(MathFunction::Log).map_err(|error| language_error(request.descriptor.id, error.to_string()))?;
	emitter.emit_stage([
		KernelEmission {
			inputs: vec![row_sums],
			outputs: vec![log_sums],
			kind: PrimitiveKind::Elementwise(Elementwise { program: logarithm }),
		},
		KernelEmission {
			inputs: vec![selected_logits, row_maxima, log_sums],
			outputs: vec![log_probabilities.id],
			kind: PrimitiveKind::Elementwise(Elementwise {
				program: categorical_finish_program(request.descriptor.id)?,
			}),
		},
	])
}

fn emit_csr_sparse_vector(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(
		request,
		&[
			"values",
			"vector",
			"value_indices",
			"operand_indices",
			"active_mask",
			"output_base",
			"output_indices",
		],
		&["product"],
		&[
			"rows",
			"columns",
			"nonzeros",
			"row_width",
			"tables_verified",
			"output_base_zero",
			"output_indices_unique",
			"tree_lanes",
		],
	)?;
	let rows = prepared_dimension(request, "rows")?;
	let columns = prepared_dimension(request, "columns")?;
	emit_csr_padded(request, emitter, "vector", columns, rows)
}

fn emit_csr_sparse_matrix(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(
		request,
		&[
			"values",
			"matrix",
			"value_indices",
			"operand_indices",
			"active_mask",
			"output_base",
			"output_indices",
		],
		&["product"],
		&[
			"rows",
			"columns",
			"features",
			"nonzeros",
			"row_width",
			"tables_verified",
			"output_base_zero",
			"output_indices_unique",
			"tree_lanes",
		],
	)?;
	let rows = prepared_dimension(request, "rows")?;
	let columns = prepared_dimension(request, "columns")?;
	let features = prepared_dimension(request, "features")?;
	let operand_elements = checked_product(request, &[columns, features], "dense matrix element count")?;
	let outputs = checked_product(
		request,
		&[rows, features],
		"sparse matrix product element count",
	)?;
	emit_csr_padded(request, emitter, "matrix", operand_elements, outputs)
}

fn emit_csr_padded(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>, operand_name: &str, operand_elements: u64, output_elements: u64) -> OperationResult<()> {
	let values = input(request, "values")?;
	let operand = input(request, operand_name)?;
	let value_indices = input(request, "value_indices")?;
	let operand_indices = input(request, "operand_indices")?;
	let active_mask = input(request, "active_mask")?;
	let output_base = input(request, "output_base")?;
	let output_indices = input(request, "output_indices")?;
	let product = output(request, "product")?;
	let nonzeros = prepared_dimension(request, "nonzeros")?;
	let row_width = prepared_dimension(request, "row_width")?;
	let tree_lanes = prepared_tree_lanes(request)?;
	for fact in [
		"tables_verified",
		"output_base_zero",
		"output_indices_unique",
	] {
		require_true(request, fact)?;
	}
	require_dtype(request, values, DType::F32, "values")?;
	require_shape(request, values, &[nonzeros], "values")?;
	require_dtype(request, operand, DType::F32, "dense operand")?;
	require_shape(request, operand, &[operand_elements], "dense operand")?;
	for (name, tensor) in [
		("value_indices", value_indices),
		("operand_indices", operand_indices),
		("active_mask", active_mask),
	] {
		require_dtype(request, tensor, DType::I32, name)?;
		require_shape(request, tensor, &[output_elements, row_width], name)?;
	}
	require_dtype(request, output_base, DType::F32, "output_base")?;
	require_shape(request, output_base, &[output_elements], "output_base")?;
	require_dtype(request, output_indices, DType::I32, "output_indices")?;
	require_shape(
		request,
		output_indices,
		&[output_elements],
		"output_indices",
	)?;
	require_dtype(request, product, DType::F32, "product")?;
	require_shape(request, product, &[output_elements], "product")?;

	let table_shape = shape(request, &[output_elements, row_width])?;
	let gathered_values = emitter.intermediate(DType::F32, table_shape.clone())?;
	let gathered_operand = emitter.intermediate(DType::F32, table_shape.clone())?;
	emitter.emit_stage([
		KernelEmission {
			inputs: vec![values.id, value_indices.id],
			outputs: vec![gathered_values],
			kind: gather(),
		},
		KernelEmission {
			inputs: vec![operand.id, operand_indices.id],
			outputs: vec![gathered_operand],
			kind: gather(),
		},
	])?;
	let products = emitter.intermediate(DType::F32, table_shape)?;
	emitter.emit(
		vec![gathered_values, gathered_operand, active_mask.id],
		vec![products],
		PrimitiveKind::Elementwise(Elementwise {
			program: masked_product_program(request.descriptor.id)?,
		}),
	)?;
	let row_sums = emitter.intermediate(DType::F32, shape(request, &[output_elements])?)?;
	emitter.emit_stage([
		KernelEmission {
			inputs: vec![products],
			outputs: vec![row_sums],
			kind: reduction(
				request.descriptor.id,
				ReduceOperator::Sum,
				&[1],
				false,
				ReduceResult::Value,
				tree_lanes,
			)?,
		},
		KernelEmission {
			inputs: vec![output_base.id, output_indices.id, row_sums],
			outputs: vec![product.id],
			kind: scatter(ScatterConflict::UniqueIndices),
		},
	])
}

fn emit_neighbor_aggregate(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(
		request,
		&[
			"features",
			"feature_indices",
			"active_mask",
			"degrees",
			"degree_indices",
			"output_base",
			"output_indices",
		],
		&["aggregate"],
		&[
			"nodes",
			"features_per_node",
			"row_width",
			"mean",
			"tables_verified",
			"output_base_zero",
			"output_indices_unique",
			"tree_lanes",
		],
	)?;
	let features = input(request, "features")?;
	let feature_indices = input(request, "feature_indices")?;
	let active_mask = input(request, "active_mask")?;
	let degrees = input(request, "degrees")?;
	let degree_indices = input(request, "degree_indices")?;
	let output_base = input(request, "output_base")?;
	let output_indices = input(request, "output_indices")?;
	let aggregate = output(request, "aggregate")?;
	let nodes = prepared_dimension(request, "nodes")?;
	let feature_count = prepared_dimension(request, "features_per_node")?;
	let row_width = prepared_dimension(request, "row_width")?;
	let output_elements = checked_product(request, &[nodes, feature_count], "aggregate element count")?;
	let mean = prepared_bool(request, "mean")?;
	let tree_lanes = prepared_tree_lanes(request)?;
	for fact in [
		"tables_verified",
		"output_base_zero",
		"output_indices_unique",
	] {
		require_true(request, fact)?;
	}
	for (name, tensor, expected) in [
		("features", features, &[output_elements][..]),
		("degrees", degrees, &[nodes][..]),
		("output_base", output_base, &[output_elements][..]),
		("aggregate", aggregate, &[output_elements][..]),
	] {
		require_dtype(request, tensor, DType::F32, name)?;
		require_shape(request, tensor, expected, name)?;
	}
	for (name, tensor, expected) in [
		(
			"feature_indices",
			feature_indices,
			&[output_elements, row_width][..],
		),
		(
			"active_mask",
			active_mask,
			&[output_elements, row_width][..],
		),
		("degree_indices", degree_indices, &[output_elements][..]),
		("output_indices", output_indices, &[output_elements][..]),
	] {
		require_dtype(request, tensor, DType::I32, name)?;
		require_shape(request, tensor, expected, name)?;
	}

	let feature_table_shape = shape(request, &[output_elements, row_width])?;
	let output_shape = shape(request, &[output_elements])?;
	let gathered_degrees = emitter.intermediate(DType::F32, output_shape.clone())?;
	let gathered_features = emitter.intermediate(DType::F32, feature_table_shape.clone())?;
	emitter.emit_stage([
		KernelEmission {
			inputs: vec![degrees.id, degree_indices.id],
			outputs: vec![gathered_degrees],
			kind: gather(),
		},
		KernelEmission {
			inputs: vec![features.id, feature_indices.id],
			outputs: vec![gathered_features],
			kind: gather(),
		},
	])?;
	let contributions = emitter.intermediate(DType::F32, feature_table_shape)?;
	emitter.emit(
		vec![gathered_features, active_mask.id],
		vec![contributions],
		PrimitiveKind::Elementwise(Elementwise {
			program: masked_value_program(request.descriptor.id)?,
		}),
	)?;
	let sums = emitter.intermediate(DType::F32, output_shape.clone())?;
	let normalized = emitter.intermediate(DType::F32, output_shape)?;
	emitter.emit_stage([
		KernelEmission {
			inputs: vec![contributions],
			outputs: vec![sums],
			kind: reduction(
				request.descriptor.id,
				ReduceOperator::Sum,
				&[1],
				false,
				ReduceResult::Value,
				tree_lanes,
			)?,
		},
		KernelEmission {
			inputs: vec![sums, gathered_degrees],
			outputs: vec![normalized],
			kind: PrimitiveKind::Elementwise(Elementwise {
				program: neighbor_normalize_program(request.descriptor.id, mean)?,
			}),
		},
		KernelEmission {
			inputs: vec![output_base.id, output_indices.id, normalized],
			outputs: vec![aggregate.id],
			kind: scatter(ScatterConflict::UniqueIndices),
		},
	])
}

fn emit_centroid_update(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(
		request,
		&[
			"points",
			"point_indices",
			"active_mask",
			"assignments",
			"centroid_count_indices",
			"centroid_base",
			"centroid_indices",
		],
		&["centroids", "counts"],
		&[
			"points_count",
			"dimensions",
			"centroid_count",
			"points_per_centroid",
			"contribution_table_verified",
			"centroid_count_indices_verified",
			"centroid_base_zero",
			"centroid_indices_unique",
			"assignments_verified",
			"tree_lanes",
		],
	)?;
	let points = input(request, "points")?;
	let point_indices = input(request, "point_indices")?;
	let active_mask = input(request, "active_mask")?;
	let assignments = input(request, "assignments")?;
	let count_indices = input(request, "centroid_count_indices")?;
	let centroid_base = input(request, "centroid_base")?;
	let centroid_indices = input(request, "centroid_indices")?;
	let centroids = output(request, "centroids")?;
	let counts = output(request, "counts")?;
	let points_count = prepared_dimension(request, "points_count")?;
	let dimensions = prepared_dimension(request, "dimensions")?;
	let centroid_count = prepared_dimension(request, "centroid_count")?;
	let points_per_centroid = prepared_dimension(request, "points_per_centroid")?;
	let point_elements = checked_product(request, &[points_count, dimensions], "point element count")?;
	let centroid_elements = checked_product(
		request,
		&[centroid_count, dimensions],
		"centroid element count",
	)?;
	let tree_lanes = prepared_tree_lanes(request)?;
	for fact in [
		"contribution_table_verified",
		"centroid_count_indices_verified",
		"centroid_base_zero",
		"centroid_indices_unique",
		"assignments_verified",
	] {
		require_true(request, fact)?;
	}
	require_dtype(request, points, DType::F32, "points")?;
	require_shape(request, points, &[point_elements], "points")?;
	for (name, tensor) in [
		("point_indices", point_indices),
		("active_mask", active_mask),
	] {
		require_dtype(request, tensor, DType::I32, name)?;
		require_shape(
			request,
			tensor,
			&[centroid_elements, points_per_centroid],
			name,
		)?;
	}
	require_dtype(request, assignments, DType::I32, "assignments")?;
	require_shape(request, assignments, &[points_count], "assignments")?;
	for (name, tensor) in [
		("centroid_count_indices", count_indices),
		("centroid_indices", centroid_indices),
	] {
		require_dtype(request, tensor, DType::I32, name)?;
		require_shape(request, tensor, &[centroid_elements], name)?;
	}
	for (name, tensor) in [("centroid_base", centroid_base), ("centroids", centroids)] {
		require_dtype(request, tensor, DType::F32, name)?;
		require_shape(request, tensor, &[centroid_elements], name)?;
	}
	require_dtype(request, counts, DType::I32, "counts")?;
	require_shape(request, counts, &[centroid_count], "counts")?;

	let table_shape = shape(request, &[centroid_elements, points_per_centroid])?;
	let gathered_points = emitter.intermediate(DType::F32, table_shape.clone())?;
	emitter.emit(
		vec![points.id, point_indices.id],
		vec![gathered_points],
		gather(),
	)?;
	let contributions = emitter.intermediate(DType::F32, table_shape)?;
	emitter.emit(
		vec![gathered_points, active_mask.id],
		vec![contributions],
		PrimitiveKind::Elementwise(Elementwise {
			program: masked_value_program(request.descriptor.id)?,
		}),
	)?;

	let flat_shape = shape(request, &[centroid_elements])?;
	let sums = emitter.intermediate(DType::F32, flat_shape.clone())?;
	let divisors = emitter.intermediate(DType::I32, flat_shape.clone())?;
	let normalized = emitter.intermediate(DType::F32, flat_shape)?;
	let bins = u32::try_from(centroid_count).map_err(|conversion_error| request_error(request, conversion_error.to_string()))?;
	emitter.emit_stage([
		KernelEmission {
			inputs: vec![contributions],
			outputs: vec![sums],
			kind: reduction(
				request.descriptor.id,
				ReduceOperator::Sum,
				&[1],
				false,
				ReduceResult::Value,
				tree_lanes,
			)?,
		},
		KernelEmission {
			inputs: vec![assignments.id],
			outputs: vec![counts.id],
			kind: PrimitiveKind::Histogram(Histogram {
				bins,
				weighted: false,
				ordering: AtomicOrdering::Relaxed,
			}),
		},
		KernelEmission {
			inputs: vec![counts.id, count_indices.id],
			outputs: vec![divisors],
			kind: gather(),
		},
		KernelEmission {
			inputs: vec![sums, divisors],
			outputs: vec![normalized],
			kind: PrimitiveKind::Elementwise(Elementwise {
				program: centroid_normalize_program(request.descriptor.id)?,
			}),
		},
		KernelEmission {
			inputs: vec![centroid_base.id, centroid_indices.id, normalized],
			outputs: vec![centroids.id],
			kind: scatter(ScatterConflict::UniqueIndices),
		},
	])
}

fn emit_core_distance(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(
		request,
		&["points", "left_indices", "right_indices", "rank_indices"],
		&["core_distances"],
		&[
			"points_count",
			"dimensions",
			"minimum_points",
			"pair_indices_verified",
			"rank_indices_verified",
			"tree_lanes",
		],
	)?;
	let points = input(request, "points")?;
	let left_indices = input(request, "left_indices")?;
	let right_indices = input(request, "right_indices")?;
	let rank_indices = input(request, "rank_indices")?;
	let core_distances = output(request, "core_distances")?;
	let points_count = prepared_dimension(request, "points_count")?;
	let dimensions = prepared_dimension(request, "dimensions")?;
	let minimum_points = prepared_dimension(request, "minimum_points")?;
	require_supported(
		request,
		points_count == 2 && minimum_points == 1,
		"the exact static core-distance graph currently supports two points and minimum_points = 1",
	)?;
	let point_elements = checked_product(request, &[points_count, dimensions], "point element count")?;
	let tree_lanes = prepared_tree_lanes(request)?;
	require_true(request, "pair_indices_verified")?;
	require_true(request, "rank_indices_verified")?;
	require_dtype(request, points, DType::F32, "points")?;
	require_shape(request, points, &[point_elements], "points")?;
	for (name, tensor) in [
		("left_indices", left_indices),
		("right_indices", right_indices),
	] {
		require_dtype(request, tensor, DType::I32, name)?;
		require_shape(request, tensor, &[2, dimensions], name)?;
	}
	require_dtype(request, rank_indices, DType::I32, "rank_indices")?;
	require_shape(request, rank_indices, &[1], "rank_indices")?;
	require_dtype(request, core_distances, DType::F32, "core_distances")?;
	require_shape(request, core_distances, &[2, 1], "core_distances")?;

	let pair_shape = shape(request, &[2, dimensions])?;
	let left = emitter.intermediate(DType::F32, pair_shape.clone())?;
	let right = emitter.intermediate(DType::F32, pair_shape.clone())?;
	let squared_differences = emitter.intermediate(DType::F32, pair_shape)?;
	let squared_distances = emitter.intermediate(DType::F32, shape(request, &[2, 1])?)?;
	let distances = emitter.intermediate(DType::F32, shape(request, &[2, 1])?)?;
	emitter.emit_stage([
		KernelEmission {
			inputs: vec![points.id, left_indices.id],
			outputs: vec![left],
			kind: gather(),
		},
		KernelEmission {
			inputs: vec![points.id, right_indices.id],
			outputs: vec![right],
			kind: gather(),
		},
		KernelEmission {
			inputs: vec![left, right],
			outputs: vec![squared_differences],
			kind: PrimitiveKind::Elementwise(Elementwise {
				program: squared_difference_program(request.descriptor.id)?,
			}),
		},
		KernelEmission {
			inputs: vec![squared_differences],
			outputs: vec![squared_distances],
			kind: reduction(
				request.descriptor.id,
				ReduceOperator::Sum,
				&[1],
				true,
				ReduceResult::Value,
				tree_lanes,
			)?,
		},
		KernelEmission {
			inputs: vec![squared_distances],
			outputs: vec![distances],
			kind: PrimitiveKind::Elementwise(Elementwise {
				program: nonnegative_square_root_program(request.descriptor.id)?,
			}),
		},
	])?;
	let sorted = emitter.intermediate(DType::F32, shape(request, &[2, 1])?)?;
	emitter.emit(
		vec![distances],
		vec![sorted],
		PrimitiveKind::Sort(Sort {
			axis: 1,
			direction: SortDirection::Ascending,
			stable: true,
			emit_indices: false,
		}),
	)?;
	emitter.emit(
		vec![sorted, rank_indices.id],
		vec![core_distances.id],
		PrimitiveKind::Gather(Gather {
			axis: 1,
			bounds: IndexBounds::Reject,
		}),
	)
}

fn emit_pairwise_l2(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(request, &["query", "training"], &["distances"], &[
		"queries",
		"training_rows",
		"dimensions",
		"tree_lanes",
	])?;
	let query = input(request, "query")?;
	let training = input(request, "training")?;
	let distances = output(request, "distances")?;
	let queries = prepared_dimension(request, "queries")?;
	let training_rows = prepared_dimension(request, "training_rows")?;
	let dimensions = prepared_dimension(request, "dimensions")?;
	let tree_lanes = prepared_tree_lanes(request)?;
	require_dtype(request, query, DType::F32, "query")?;
	require_dtype(request, training, DType::F32, "training")?;
	require_dtype(request, distances, DType::F32, "distances")?;
	require_shape(request, query, &[queries, dimensions], "query")?;
	require_shape(request, training, &[training_rows, dimensions], "training")?;
	require_shape(request, distances, &[queries, training_rows], "distances")?;

	let query_squared = emitter.intermediate(DType::F32, query.shape.clone())?;
	let training_squared = emitter.intermediate(DType::F32, training.shape.clone())?;
	emitter.emit_stage([
		KernelEmission {
			inputs: vec![query.id],
			outputs: vec![query_squared],
			kind: PrimitiveKind::Elementwise(Elementwise {
				program: square_program(request.descriptor.id)?,
			}),
		},
		KernelEmission {
			inputs: vec![training.id],
			outputs: vec![training_squared],
			kind: PrimitiveKind::Elementwise(Elementwise {
				program: square_program(request.descriptor.id)?,
			}),
		},
	])?;
	let query_norms = emitter.intermediate(DType::F32, shape(request, &[queries, 1])?)?;
	let training_norms = emitter.intermediate(DType::F32, shape(request, &[training_rows])?)?;
	emitter.emit_stage([
		KernelEmission {
			inputs: vec![query_squared],
			outputs: vec![query_norms],
			kind: reduction(
				request.descriptor.id,
				ReduceOperator::Sum,
				&[1],
				true,
				ReduceResult::Value,
				tree_lanes,
			)?,
		},
		KernelEmission {
			inputs: vec![training_squared],
			outputs: vec![training_norms],
			kind: reduction(
				request.descriptor.id,
				ReduceOperator::Sum,
				&[1],
				false,
				ReduceResult::Value,
				tree_lanes,
			)?,
		},
	])?;
	let products = emitter.intermediate(DType::F32, distances.shape.clone())?;
	emitter.emit(
		vec![query.id, training.id],
		vec![products],
		PrimitiveKind::Contraction(Contraction {
			batch_axes: Vec::new(),
			contract_axes: vec![(1, 1)],
		}),
	)?;
	emitter.emit(
		vec![query_norms, training_norms, products],
		vec![distances.id],
		PrimitiveKind::Elementwise(Elementwise {
			program: pairwise_l2_result_program(request.descriptor.id)?,
		}),
	)
}

fn emit_fixed_radius_singleton(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(
		request,
		&[
			"points",
			"counts",
			"row_pointer_base",
			"row_pointer_indices",
			"row_pointer_updates",
			"neighbor_base",
			"neighbor_indices",
		],
		&["row_pointers", "neighbors"],
		&[
			"points_count",
			"dimensions",
			"epsilon",
			"singleton_tables_verified",
			"bases_zero",
			"destination_indices_unique",
			"tree_lanes",
		],
	)?;
	let points = input(request, "points")?;
	let counts = input(request, "counts")?;
	let row_pointer_base = input(request, "row_pointer_base")?;
	let row_pointer_indices = input(request, "row_pointer_indices")?;
	let row_pointer_updates = input(request, "row_pointer_updates")?;
	let neighbor_base = input(request, "neighbor_base")?;
	let neighbor_indices = input(request, "neighbor_indices")?;
	let row_pointers = output(request, "row_pointers")?;
	let neighbors = output(request, "neighbors")?;
	let points_count = prepared_dimension(request, "points_count")?;
	let dimensions = prepared_dimension(request, "dimensions")?;
	require_supported(
		request,
		points_count == 1,
		"the finite exact fixed-radius graph currently supports a singleton point set",
	)?;
	let epsilon = prepared_f32(request, "epsilon")?;
	require_request_fact(
		request,
		epsilon.is_finite() && epsilon >= 0.0,
		"epsilon must be a finite nonnegative f32",
	)?;
	let tree_lanes = prepared_tree_lanes(request)?;
	for fact in [
		"singleton_tables_verified",
		"bases_zero",
		"destination_indices_unique",
	] {
		require_true(request, fact)?;
	}
	require_dtype(request, points, DType::F32, "points")?;
	require_shape(request, points, &[dimensions], "points")?;
	for (name, tensor, expected) in [
		("counts", counts, &[1][..]),
		("row_pointer_base", row_pointer_base, &[2][..]),
		("row_pointer_indices", row_pointer_indices, &[2][..]),
		("row_pointer_updates", row_pointer_updates, &[2][..]),
		("neighbor_base", neighbor_base, &[1][..]),
		("neighbor_indices", neighbor_indices, &[1][..]),
		("row_pointers", row_pointers, &[2][..]),
		("neighbors", neighbors, &[1][..]),
	] {
		require_dtype(request, tensor, DType::I32, name)?;
		require_shape(request, tensor, expected, name)?;
	}

	let offsets = emitter.intermediate(DType::I32, counts.shape.clone())?;
	emitter.emit(
		vec![counts.id],
		vec![offsets],
		PrimitiveKind::Scan(Scan {
			operator: ReduceOperator::Sum,
			axis: 0,
			mode: ScanMode::Exclusive {
				identity: ScalarLiteral::I32(0),
			},
			reverse: false,
			tree_lanes,
		}),
	)?;
	let mapped_offsets = emitter.intermediate(DType::I32, counts.shape.clone())?;
	emitter.emit(
		vec![offsets],
		vec![mapped_offsets],
		PrimitiveKind::Elementwise(Elementwise {
			program: identity_program(request.descriptor.id, DType::I32)?,
		}),
	)?;
	emitter.emit_stage([
		KernelEmission {
			inputs: vec![
				row_pointer_base.id,
				row_pointer_indices.id,
				row_pointer_updates.id,
			],
			outputs: vec![row_pointers.id],
			kind: scatter(ScatterConflict::UniqueIndices),
		},
		KernelEmission {
			inputs: vec![neighbor_base.id, neighbor_indices.id, mapped_offsets],
			outputs: vec![neighbors.id],
			kind: scatter(ScatterConflict::UniqueIndices),
		},
	])
}

fn emit_union_find_two_nodes(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(
		request,
		&[
			"parent_base",
			"edge_sources",
			"edge_destinations",
			"node_indices",
			"labels_base",
		],
		&["labels"],
		&[
			"nodes",
			"edges",
			"two_node_topology_verified",
			"parent_base_verified",
			"node_indices_verified",
			"labels_base_zero",
		],
	)?;
	require_iteration_input(request, "parent_base")?;
	let parent_base = input(request, "parent_base")?;
	let edge_sources = input(request, "edge_sources")?;
	let edge_destinations = input(request, "edge_destinations")?;
	let node_indices = input(request, "node_indices")?;
	let labels_base = input(request, "labels_base")?;
	let labels = output(request, "labels")?;
	let nodes = prepared_dimension(request, "nodes")?;
	require_supported(
		request,
		nodes == 2,
		"the exact bounded union-find graph currently supports two nodes",
	)?;
	let edges = prepared_dimension(request, "edges")?;
	for fact in [
		"two_node_topology_verified",
		"parent_base_verified",
		"node_indices_verified",
		"labels_base_zero",
	] {
		require_true(request, fact)?;
	}
	for (name, tensor, expected) in [
		("parent_base", parent_base, &[2][..]),
		("edge_sources", edge_sources, &[edges][..]),
		("edge_destinations", edge_destinations, &[edges][..]),
		("node_indices", node_indices, &[2][..]),
		("labels_base", labels_base, &[2][..]),
		("labels", labels, &[2][..]),
	] {
		require_dtype(request, tensor, DType::I32, name)?;
		require_shape(request, tensor, expected, name)?;
	}

	let source_roots = emitter.intermediate(DType::I32, edge_sources.shape.clone())?;
	let destination_roots = emitter.intermediate(DType::I32, edge_sources.shape.clone())?;
	emitter.emit_stage([
		KernelEmission {
			inputs: vec![parent_base.id, edge_sources.id],
			outputs: vec![source_roots],
			kind: gather(),
		},
		KernelEmission {
			inputs: vec![parent_base.id, edge_destinations.id],
			outputs: vec![destination_roots],
			kind: gather(),
		},
	])?;
	let hook_destinations = emitter.intermediate(DType::I32, edge_sources.shape.clone())?;
	let hook_updates = emitter.intermediate(DType::I32, edge_sources.shape.clone())?;
	emitter.emit(
		vec![source_roots, destination_roots],
		vec![hook_destinations, hook_updates],
		PrimitiveKind::Elementwise(Elementwise {
			program: minimum_representative_hook_program(request.descriptor.id)?,
		}),
	)?;
	let hooked = emitter.intermediate(DType::I32, parent_base.shape.clone())?;
	let compressed = emitter.intermediate(DType::I32, parent_base.shape.clone())?;
	emitter.emit_stage([
		KernelEmission {
			inputs: vec![parent_base.id, hook_destinations, hook_updates],
			outputs: vec![hooked],
			kind: scatter(ScatterConflict::Atomic {
				operation: AtomicOperation::Minimum,
				ordering: AtomicOrdering::SequentiallyConsistent,
			}),
		},
		KernelEmission {
			inputs: vec![hooked, node_indices.id],
			outputs: vec![compressed],
			kind: gather(),
		},
		KernelEmission {
			inputs: vec![labels_base.id, node_indices.id, compressed],
			outputs: vec![labels.id],
			kind: scatter(ScatterConflict::UniqueIndices),
		},
	])
}

fn emit_boruvka(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(
		request,
		&[
			"parent",
			"edge_sources",
			"edge_destinations",
			"edge_weights",
			"edge_indices",
			"mask_base",
			"unit_update",
		],
		&["in_mst", "total_weight"],
		&[
			"nodes",
			"edges",
			"two_node_topology_verified",
			"parent_verified",
			"edge_indices_verified",
			"edge_weights_finite",
			"mask_base_zero",
			"tree_lanes",
		],
	)?;
	require_iteration_input(request, "parent")?;
	let parent = input(request, "parent")?;
	let edge_sources = input(request, "edge_sources")?;
	let edge_destinations = input(request, "edge_destinations")?;
	let edge_weights = input(request, "edge_weights")?;
	let edge_indices = input(request, "edge_indices")?;
	let mask_base = input(request, "mask_base")?;
	let unit_update = input(request, "unit_update")?;
	let in_mst = output(request, "in_mst")?;
	let total_weight = output(request, "total_weight")?;
	let nodes = prepared_dimension(request, "nodes")?;
	require_supported(
		request,
		nodes == 2,
		"the exact finite Boruvka graph currently supports a two-node topology",
	)?;
	let edges = prepared_dimension(request, "edges")?;
	let tree_lanes = prepared_tree_lanes(request)?;
	for fact in [
		"two_node_topology_verified",
		"parent_verified",
		"edge_indices_verified",
		"edge_weights_finite",
		"mask_base_zero",
	] {
		require_true(request, fact)?;
	}
	for (name, tensor, expected) in [
		("parent", parent, &[2][..]),
		("edge_sources", edge_sources, &[edges][..]),
		("edge_destinations", edge_destinations, &[edges][..]),
		("edge_indices", edge_indices, &[edges][..]),
		("mask_base", mask_base, &[edges][..]),
		("unit_update", unit_update, &[1][..]),
		("in_mst", in_mst, &[edges][..]),
	] {
		require_dtype(request, tensor, DType::I32, name)?;
		require_shape(request, tensor, expected, name)?;
	}
	require_dtype(request, edge_weights, DType::F32, "edge_weights")?;
	require_shape(request, edge_weights, &[edges], "edge_weights")?;
	require_dtype(request, total_weight, DType::F32, "total_weight")?;
	require_shape(request, total_weight, &[1], "total_weight")?;

	let source_components = emitter.intermediate(DType::I32, edge_sources.shape.clone())?;
	let destination_components = emitter.intermediate(DType::I32, edge_sources.shape.clone())?;
	let gathered_weights = emitter.intermediate(DType::F32, edge_weights.shape.clone())?;
	emitter.emit_stage([
		KernelEmission {
			inputs: vec![parent.id, edge_sources.id],
			outputs: vec![source_components],
			kind: gather(),
		},
		KernelEmission {
			inputs: vec![parent.id, edge_destinations.id],
			outputs: vec![destination_components],
			kind: gather(),
		},
		KernelEmission {
			inputs: vec![edge_weights.id, edge_indices.id],
			outputs: vec![gathered_weights],
			kind: gather(),
		},
	])?;
	let minimum_weight = emitter.intermediate(DType::F32, shape(request, &[1])?)?;
	let minimum_edge = emitter.intermediate(DType::I32, shape(request, &[1])?)?;
	emitter.emit(
		vec![gathered_weights],
		vec![minimum_weight, minimum_edge],
		reduction(
			request.descriptor.id,
			ReduceOperator::Minimum,
			&[0],
			false,
			ReduceResult::ValueAndIndex,
			tree_lanes,
		)?,
	)?;
	emitter.emit_stage([
		KernelEmission {
			inputs: vec![minimum_weight],
			outputs: vec![total_weight.id],
			kind: PrimitiveKind::Elementwise(Elementwise {
				program: identity_program(request.descriptor.id, DType::F32)?,
			}),
		},
		KernelEmission {
			inputs: vec![mask_base.id, minimum_edge, unit_update.id],
			outputs: vec![in_mst.id],
			kind: scatter(ScatterConflict::UniqueIndices),
		},
	])
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

fn reduction(operation: OperationId, operator: ReduceOperator, axes: &[usize], keep_dimensions: bool, result: ReduceResult, tree_lanes: u32) -> OperationResult<PrimitiveKind> {
	let axes = AxisSet::new(axes.to_vec()).map_err(|error| language_error(operation, error.to_string()))?;
	Ok(PrimitiveKind::Reduce(Reduce {
		operator,
		axes,
		keep_dimensions,
		result,
		tree_lanes,
	}))
}

fn shape(request: &MaterializationRequest<'_>, extents: &[u64]) -> OperationResult<Shape> { Shape::new(extents.to_vec()).map_err(|error| language_error(request.descriptor.id, error.to_string())) }

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

fn prepared_count(request: &MaterializationRequest<'_>, name: &str) -> OperationResult<u64> {
	let value = prepared_u64(request.descriptor.id, request.parameters, name)?;
	match value <= MAX_I32_INDEX {
		true => Ok(value),
		false => {
			Err(unsupported(
				request,
				format!("{name} must be in 0..={MAX_I32_INDEX}, got {value}"),
			))
		}
	}
}

fn checked_product(request: &MaterializationRequest<'_>, factors: &[u64], role: &str) -> OperationResult<u64> {
	let product = factors.iter().try_fold(1_u64, |product, factor| {
		product.checked_mul(*factor).ok_or_else(|| {
			operation_error(
				request.descriptor.id,
				OperationErrorKind::WorkspaceArithmeticOverflow,
				format!("{role} overflowed u64"),
			)
		})
	})?;
	match product <= MAX_I32_INDEX {
		true => Ok(product),
		false => {
			Err(unsupported(
				request,
				format!("{role} {product} exceeds the canonical int32 index domain"),
			))
		}
	}
}

fn finite_parameter(request: &MaterializationRequest<'_>, name: &str) -> OperationResult<f32> {
	let value = prepared_f32(request, name)?;
	match value.is_finite() {
		true => Ok(value),
		false => {
			Err(request_error(
				request,
				format!("{name} must be a finite f32"),
			))
		}
	}
}

fn prepared_bool(request: &MaterializationRequest<'_>, name: &str) -> OperationResult<bool> {
	match request.parameters.get(name) {
		Some(super::PreparedParameter::Bool(value)) => Ok(*value),
		Some(value) => {
			Err(operation_error(
				request.descriptor.id,
				OperationErrorKind::PreparedParameterTypeMismatch,
				format!("prepared parameter {name:?} is {value:?}, expected Bool"),
			))
		}
		None => {
			Err(operation_error(
				request.descriptor.id,
				OperationErrorKind::MissingPreparedParameter,
				format!("prepared parameter {name:?} is required"),
			))
		}
	}
}

fn require_iteration_input(request: &MaterializationRequest<'_>, expected: &str) -> OperationResult<()> {
	match request.iteration_shape_input == expected {
		true => Ok(()),
		false => {
			Err(request_error(
				request,
				format!("iteration shape input must be {expected:?} for the statically bounded repeated graph"),
			))
		}
	}
}

fn unsupported(request: &MaterializationRequest<'_>, detail: impl Into<String>) -> crate::OperationError {
	operation_error(
		request.descriptor.id,
		OperationErrorKind::UnsupportedConcreteShape,
		detail,
	)
}

fn require_supported(request: &MaterializationRequest<'_>, condition: bool, detail: &'static str) -> OperationResult<()> {
	match condition {
		true => Ok(()),
		false => Err(unsupported(request, detail)),
	}
}

fn require_request_fact(request: &MaterializationRequest<'_>, condition: bool, detail: &'static str) -> OperationResult<()> {
	match condition {
		true => Ok(()),
		false => Err(request_error(request, detail)),
	}
}

fn bool_mask(operation: OperationId, builder: &mut recipe_language::ScalarProgramBuilder, mask: recipe_language::ScalarExpression) -> OperationResult<recipe_language::ScalarExpression> {
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

fn temporal_difference_program(operation: OperationId, gamma: f32) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let reward = scalar_input(operation, &mut builder, DType::F32)?;
	let next_value = scalar_input(operation, &mut builder, DType::F32)?;
	let done = scalar_input(operation, &mut builder, DType::I32)?;
	let done = bool_mask(operation, &mut builder, done)?;
	let done = scalar_unary(operation, &mut builder, ScalarOpcode::ConvertI32ToF32, done)?;
	let one = scalar_f32(operation, &mut builder, 1.0)?;
	let gamma = scalar_f32(operation, &mut builder, gamma)?;
	let live = scalar_binary(operation, &mut builder, ScalarOpcode::Subtract, one, done)?;
	let discounted = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		next_value,
		gamma,
	)?;
	let continuation = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		discounted,
		live,
	)?;
	let target = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Add,
		reward,
		continuation,
	)?;
	scalar_finish(operation, builder, &[target])
}

fn gcn_normalization_program(operation: OperationId) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let feature = scalar_input(operation, &mut builder, DType::F32)?;
	let degree = scalar_input(operation, &mut builder, DType::F32)?;
	let zero = scalar_f32(operation, &mut builder, 0.0)?;
	let nonnegative = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::GreaterThanOrEqual,
		degree,
		zero,
	)?;
	scalar_unary(operation, &mut builder, ScalarOpcode::Require, nonnegative)?;
	let positive = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::GreaterThan,
		degree,
		zero,
	)?;
	let root = scalar_unary(operation, &mut builder, ScalarOpcode::SquareRoot, degree)?;
	let one = scalar_f32(operation, &mut builder, 1.0)?;
	let inverse_root = scalar_binary(operation, &mut builder, ScalarOpcode::Divide, one, root)?;
	let scaled = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		feature,
		inverse_root,
	)?;
	let normalized = scalar_ternary(
		operation,
		&mut builder,
		ScalarOpcode::Select,
		positive,
		scaled,
		feature,
	)?;
	scalar_finish(operation, builder, &[normalized])
}

fn advantage_delta_program(operation: OperationId, gamma: f32) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let reward = scalar_input(operation, &mut builder, DType::F32)?;
	let value = scalar_input(operation, &mut builder, DType::F32)?;
	let next_value = scalar_input(operation, &mut builder, DType::F32)?;
	let has_next = scalar_input(operation, &mut builder, DType::I32)?;
	let has_next = bool_mask(operation, &mut builder, has_next)?;
	let zero = scalar_f32(operation, &mut builder, 0.0)?;
	let gamma = scalar_f32(operation, &mut builder, gamma)?;
	let next = scalar_ternary(
		operation,
		&mut builder,
		ScalarOpcode::Select,
		has_next,
		next_value,
		zero,
	)?;
	let discounted = scalar_binary(operation, &mut builder, ScalarOpcode::Multiply, gamma, next)?;
	let reward_plus_next = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Add,
		reward,
		discounted,
	)?;
	let delta = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Subtract,
		reward_plus_next,
		value,
	)?;
	scalar_finish(operation, builder, &[delta])
}

fn gaussian_term_program(operation: OperationId) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let mean = scalar_input(operation, &mut builder, DType::F32)?;
	let log_standard_deviation = scalar_input(operation, &mut builder, DType::F32)?;
	let action = scalar_input(operation, &mut builder, DType::F32)?;
	let standard_deviation = scalar_input(operation, &mut builder, DType::F32)?;
	let difference = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Subtract,
		action,
		mean,
	)?;
	let z = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Divide,
		difference,
		standard_deviation,
	)?;
	let negative_half = scalar_f32(operation, &mut builder, -0.5)?;
	let negative_half_z = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		negative_half,
		z,
	)?;
	let quadratic = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		negative_half_z,
		z,
	)?;
	let without_scale = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Subtract,
		quadratic,
		log_standard_deviation,
	)?;
	let half_log_two_pi = scalar_f32(operation, &mut builder, 0.918_938_5)?;
	let term = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Subtract,
		without_scale,
		half_log_two_pi,
	)?;
	scalar_finish(operation, builder, &[term])
}

fn subtract_program(operation: OperationId) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let left = scalar_input(operation, &mut builder, DType::F32)?;
	let right = scalar_input(operation, &mut builder, DType::F32)?;
	let result = scalar_binary(operation, &mut builder, ScalarOpcode::Subtract, left, right)?;
	scalar_finish(operation, builder, &[result])
}

fn checked_flat_action_program(operation: OperationId, action_count: u64) -> OperationResult<recipe_core::ScalarProgram> {
	let action_count = i32::try_from(action_count).map_err(|error| {
		operation_error(
			operation,
			OperationErrorKind::InvalidMaterializationRequest,
			error.to_string(),
		)
	})?;
	let mut builder = scalar_builder(operation)?;
	let row_base = scalar_input(operation, &mut builder, DType::I32)?;
	let action = scalar_input(operation, &mut builder, DType::I32)?;
	let zero = builder
		.i32(0)
		.map_err(|error| language_error(operation, error.to_string()))?;
	let limit = builder
		.i32(action_count)
		.map_err(|error| language_error(operation, error.to_string()))?;
	let nonnegative = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::GreaterThanOrEqual,
		action,
		zero,
	)?;
	let below_limit = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::LessThan,
		action,
		limit,
	)?;
	let valid = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::BitAnd,
		nonnegative,
		below_limit,
	)?;
	scalar_unary(operation, &mut builder, ScalarOpcode::Require, valid)?;
	let index = scalar_binary(operation, &mut builder, ScalarOpcode::Add, row_base, action)?;
	scalar_finish(operation, builder, &[index])
}

fn categorical_finish_program(operation: OperationId) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let selected = scalar_input(operation, &mut builder, DType::F32)?;
	let maximum = scalar_input(operation, &mut builder, DType::F32)?;
	let log_sum = scalar_input(operation, &mut builder, DType::F32)?;
	let log_normalizer = scalar_binary(operation, &mut builder, ScalarOpcode::Add, maximum, log_sum)?;
	let log_probability = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Subtract,
		selected,
		log_normalizer,
	)?;
	scalar_finish(operation, builder, &[log_probability])
}

fn masked_product_program(operation: OperationId) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let left = scalar_input(operation, &mut builder, DType::F32)?;
	let right = scalar_input(operation, &mut builder, DType::F32)?;
	let active = scalar_input(operation, &mut builder, DType::I32)?;
	let active = bool_mask(operation, &mut builder, active)?;
	let product = scalar_binary(operation, &mut builder, ScalarOpcode::Multiply, left, right)?;
	let zero = scalar_f32(operation, &mut builder, 0.0)?;
	let masked = scalar_ternary(
		operation,
		&mut builder,
		ScalarOpcode::Select,
		active,
		product,
		zero,
	)?;
	scalar_finish(operation, builder, &[masked])
}

fn neighbor_normalize_program(operation: OperationId, mean: bool) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let sum = scalar_input(operation, &mut builder, DType::F32)?;
	let degree = scalar_input(operation, &mut builder, DType::F32)?;
	let zero = scalar_f32(operation, &mut builder, 0.0)?;
	let normalized = match mean {
		true => {
			let nonnegative = scalar_binary(
				operation,
				&mut builder,
				ScalarOpcode::GreaterThanOrEqual,
				degree,
				zero,
			)?;
			scalar_unary(operation, &mut builder, ScalarOpcode::Require, nonnegative)?;
			let positive = scalar_binary(
				operation,
				&mut builder,
				ScalarOpcode::GreaterThan,
				degree,
				zero,
			)?;
			let quotient = scalar_binary(operation, &mut builder, ScalarOpcode::Divide, sum, degree)?;
			scalar_ternary(
				operation,
				&mut builder,
				ScalarOpcode::Select,
				positive,
				quotient,
				sum,
			)?
		}
		false => sum,
	};
	scalar_finish(operation, builder, &[normalized])
}

fn masked_value_program(operation: OperationId) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let value = scalar_input(operation, &mut builder, DType::F32)?;
	let active = scalar_input(operation, &mut builder, DType::I32)?;
	let active = bool_mask(operation, &mut builder, active)?;
	let zero = scalar_f32(operation, &mut builder, 0.0)?;
	let value = scalar_ternary(
		operation,
		&mut builder,
		ScalarOpcode::Select,
		active,
		value,
		zero,
	)?;
	scalar_finish(operation, builder, &[value])
}

fn centroid_normalize_program(operation: OperationId) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let sum = scalar_input(operation, &mut builder, DType::F32)?;
	let count = scalar_input(operation, &mut builder, DType::I32)?;
	let zero_i32 = builder
		.i32(0)
		.map_err(|error| language_error(operation, error.to_string()))?;
	let nonnegative = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::GreaterThanOrEqual,
		count,
		zero_i32,
	)?;
	scalar_unary(operation, &mut builder, ScalarOpcode::Require, nonnegative)?;
	let positive = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::GreaterThan,
		count,
		zero_i32,
	)?;
	let count = scalar_unary(
		operation,
		&mut builder,
		ScalarOpcode::ConvertI32ToF32,
		count,
	)?;
	let quotient = scalar_binary(operation, &mut builder, ScalarOpcode::Divide, sum, count)?;
	let zero = scalar_f32(operation, &mut builder, 0.0)?;
	let centroid = scalar_ternary(
		operation,
		&mut builder,
		ScalarOpcode::Select,
		positive,
		quotient,
		zero,
	)?;
	scalar_finish(operation, builder, &[centroid])
}

fn squared_difference_program(operation: OperationId) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let left = scalar_input(operation, &mut builder, DType::F32)?;
	let right = scalar_input(operation, &mut builder, DType::F32)?;
	let difference = scalar_binary(operation, &mut builder, ScalarOpcode::Subtract, left, right)?;
	let squared = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		difference,
		difference,
	)?;
	scalar_finish(operation, builder, &[squared])
}

fn square_program(operation: OperationId) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let value = scalar_input(operation, &mut builder, DType::F32)?;
	let squared = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		value,
		value,
	)?;
	scalar_finish(operation, builder, &[squared])
}

fn nonnegative_square_root_program(operation: OperationId) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let value = scalar_input(operation, &mut builder, DType::F32)?;
	let zero = scalar_f32(operation, &mut builder, 0.0)?;
	let valid = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::GreaterThanOrEqual,
		value,
		zero,
	)?;
	scalar_unary(operation, &mut builder, ScalarOpcode::Require, valid)?;
	let root = scalar_unary(operation, &mut builder, ScalarOpcode::SquareRoot, value)?;
	scalar_finish(operation, builder, &[root])
}

fn pairwise_l2_result_program(operation: OperationId) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let query_norm = scalar_input(operation, &mut builder, DType::F32)?;
	let training_norm = scalar_input(operation, &mut builder, DType::F32)?;
	let product = scalar_input(operation, &mut builder, DType::F32)?;
	let two = scalar_f32(operation, &mut builder, 2.0)?;
	let twice_product = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		product,
		two,
	)?;
	let norm_sum = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Add,
		query_norm,
		training_norm,
	)?;
	let squared_distance = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Subtract,
		norm_sum,
		twice_product,
	)?;
	let zero = scalar_f32(operation, &mut builder, 0.0)?;
	let nonnegative = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Maximum,
		squared_distance,
		zero,
	)?;
	let distance = scalar_unary(
		operation,
		&mut builder,
		ScalarOpcode::SquareRoot,
		nonnegative,
	)?;
	scalar_finish(operation, builder, &[distance])
}

fn minimum_representative_hook_program(operation: OperationId) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let source = scalar_input(operation, &mut builder, DType::I32)?;
	let destination = scalar_input(operation, &mut builder, DType::I32)?;
	let hook_destination = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Maximum,
		source,
		destination,
	)?;
	let hook_update = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Minimum,
		source,
		destination,
	)?;
	scalar_finish(operation, builder, &[hook_destination, hook_update])
}
