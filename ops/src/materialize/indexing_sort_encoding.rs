use recipe_core::{DType, ScalarOpcode};
use recipe_language::{
	AxisSet, Elementwise, Gather, IndexBounds, IndexMap, PrimitiveKind, Reduce, ReduceOperator, ReduceResult, Scan,
	ScanMode, Scatter, ScatterConflict, Shape, Sort, SortDirection,
};

use super::{
	Emitter, FamilyDispatch, KernelEmission, MAX_I32_INDEX, MaterializationRequest, exact_f32_extent,
	identity_program, input, language_error, operation_error, output, prepared_u64, request_error, require_dtype,
	require_exact_abi, require_shape, require_true, scalar_binary, scalar_builder, scalar_f32, scalar_finish,
	scalar_input, scalar_ternary, scalar_unary,
};
use crate::{OperationDescriptor, OperationErrorKind, OperationResult};

const OPERATIONS: &[(&str, &str)] = &[
	("gpu_add_col", "gpu-core/src/kernels.rs:6839"),
	("gpu_add_col_scaled_inplace", "gpu-core/src/kernels.rs:4525"),
	("gpu_add_diag", "gpu-core/src/kernels.rs:2298"),
	("gpu_argsort", "gpu-core/src/reductions.rs:458"),
	("gpu_bin_edges_uniform", "gpu-core/src/encoding.rs:96"),
	("gpu_concat_into", "gpu-core/src/kernels.rs:5361"),
	("gpu_one_hot", "gpu-core/src/encoding.rs:169"),
	("gpu_pack_upper_tri", "gpu-core/src/kernels.rs:5563"),
	("gpu_partial_argsort", "gpu-core/src/kernels.rs:5267"),
	("gpu_segment_sum", "gpu-core/src/reductions.rs:627"),
	("gpu_slice_cols", "gpu-core/src/kernels.rs:6550"),
	("gpu_slice_lead_into", "gpu-core/src/kernels.rs:5390"),
	("gpu_slice_rows", "gpu-core/src/kernels.rs:5619"),
	("gpu_topk_per_row", "gpu-core/src/kernels.rs:4712"),
	("gpu_transpose", "gpu-core/src/kernels.rs:5540"),
	("gpu_tril_mask", "gpu-core/src/kernels.rs:6578"),
	("gpu_vconcat", "gpu-core/src/kernels.rs:6514"),
];

pub(super) fn supports(descriptor: OperationDescriptor) -> bool {
	OPERATIONS.contains(&(descriptor.symbol, descriptor.source))
}

pub(super) fn dispatch(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> FamilyDispatch {
	let result = match supports(request.descriptor) {
		false => return FamilyDispatch::NotOwned,
		true => {
			match request.descriptor.symbol {
				"gpu_add_col" => emit_column_update(request, emitter, ColumnUpdate::Add),
				"gpu_add_col_scaled_inplace" => emit_column_update(request, emitter, ColumnUpdate::ScaledAdd),
				"gpu_add_diag" => emit_diagonal_add(request, emitter),
				"gpu_argsort" => emit_argsort(request, emitter),
				"gpu_bin_edges_uniform" => emit_uniform_edges_single_row(request, emitter),
				"gpu_concat_into" => emit_row_concatenation(request, emitter),
				"gpu_one_hot" => emit_one_hot(request, emitter),
				"gpu_pack_upper_tri" => emit_dense_upper_triangle(request, emitter),
				"gpu_partial_argsort" => emit_partial_argsort(request, emitter),
				"gpu_segment_sum" => emit_segment_sum(request, emitter),
				"gpu_slice_cols" => emit_column_slice(request, emitter),
				"gpu_slice_lead_into" => emit_leading_slice(request, emitter),
				"gpu_slice_rows" => emit_row_slice(request, emitter),
				"gpu_topk_per_row" => emit_topk_per_row(request, emitter),
				"gpu_transpose" => emit_transpose(request, emitter),
				"gpu_tril_mask" => emit_triangular_mask(request, emitter),
				"gpu_vconcat" => emit_vector_concatenation(request, emitter),
				symbol => {
					Err(operation_error(
						request.descriptor.id,
						OperationErrorKind::GraphMaterializationFailed,
						format!("indexing/sort/encoding dispatch is incomplete for {symbol}"),
					))
				}
			}
		}
	};
	FamilyDispatch::Owned(result)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ColumnUpdate {
	Add,
	ScaledAdd,
}

#[derive(Clone, Copy, Debug)]
struct SortedPrefix<'a> {
	elements: u64,
	prefix: u64,
	axis: usize,
	prefix_name: &'a str,
	output_name: &'a str,
	verification_fact: &'a str,
}

fn emit_column_update(
	request: &MaterializationRequest<'_>,
	emitter: &mut Emitter<'_>,
	update: ColumnUpdate,
) -> OperationResult<()> {
	let input_names: &[&str] = match update {
		ColumnUpdate::Add => &["matrix", "column", "column_indices"],
		ColumnUpdate::ScaledAdd => &["matrix", "column", "scale", "column_indices"],
	};
	require_exact_abi(request, input_names, &["updated_matrix"], &[
		"rows",
		"columns",
		"column_index",
		"column_indices_verified_unique",
	])?;
	let matrix = input(request, "matrix")?;
	let column = input(request, "column")?;
	let indices = input(request, "column_indices")?;
	let updated = output(request, "updated_matrix")?;
	let rows = positive_parameter(request, "rows")?;
	let columns = positive_parameter(request, "columns")?;
	let selected = prepared_u64(request.descriptor.id, request.parameters, "column_index")?;
	match selected < columns {
		true => Ok(()),
		false => {
			Err(request_error(
				request,
				"column_index must be smaller than columns",
			))
		}
	}?;
	let elements = checked_product(request, rows, columns, "matrix element count")?;
	require_i32_indexable(request, elements, "matrix element count")?;
	require_true(request, "column_indices_verified_unique")?;
	require_dtype(request, matrix, DType::F32, "matrix")?;
	require_dtype(request, column, DType::F32, "column")?;
	require_dtype(request, indices, DType::I32, "column_indices")?;
	require_dtype(request, updated, DType::F32, "updated_matrix")?;
	require_shape(request, matrix, &[elements], "matrix")?;
	require_shape(request, column, &[rows], "column")?;
	require_shape(request, indices, &[rows], "column_indices")?;
	require_shape(request, updated, &[elements], "updated_matrix")?;

	let selected_values = emitter.intermediate(DType::F32, column.shape.clone())?;
	emitter.emit(
		vec![matrix.id, indices.id],
		vec![selected_values],
		checked_gather(0),
	)?;
	let updates = emitter.intermediate(DType::F32, column.shape.clone())?;
	let program = match update {
		ColumnUpdate::Add => add_program(request)?,
		ColumnUpdate::ScaledAdd => scaled_add_program(request)?,
	};
	let map_inputs = match update {
		ColumnUpdate::Add => vec![selected_values, column.id],
		ColumnUpdate::ScaledAdd => {
			let scale = input(request, "scale")?;
			require_dtype(request, scale, DType::F32, "scale")?;
			require_shape(request, scale, &[1], "scale")?;
			vec![selected_values, column.id, scale.id]
		}
	};
	emitter.emit(
		map_inputs,
		vec![updates],
		PrimitiveKind::Elementwise(Elementwise { program }),
	)?;
	emitter.emit(
		vec![matrix.id, indices.id, updates],
		vec![updated.id],
		checked_unique_scatter(0),
	)
}

fn emit_diagonal_add(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(
		request,
		&["matrix", "diagonal_value", "diagonal_indices"],
		&["updated_matrix"],
		&["dimension", "diagonal_indices_verified_unique"],
	)?;
	let matrix = input(request, "matrix")?;
	let value = input(request, "diagonal_value")?;
	let indices = input(request, "diagonal_indices")?;
	let updated = output(request, "updated_matrix")?;
	let dimension = positive_parameter(request, "dimension")?;
	let elements = checked_product(request, dimension, dimension, "square matrix element count")?;
	require_i32_indexable(request, elements, "square matrix element count")?;
	require_true(request, "diagonal_indices_verified_unique")?;
	require_dtype(request, matrix, DType::F32, "matrix")?;
	require_dtype(request, value, DType::F32, "diagonal_value")?;
	require_dtype(request, indices, DType::I32, "diagonal_indices")?;
	require_dtype(request, updated, DType::F32, "updated_matrix")?;
	require_shape(request, matrix, &[elements], "matrix")?;
	require_shape(request, value, &[1], "diagonal_value")?;
	require_shape(request, indices, &[dimension], "diagonal_indices")?;
	require_shape(request, updated, &[elements], "updated_matrix")?;

	let diagonal = emitter.intermediate(DType::F32, indices.shape.clone())?;
	emitter.emit(
		vec![matrix.id, indices.id],
		vec![diagonal],
		checked_gather(0),
	)?;
	let updates = emitter.intermediate(DType::F32, indices.shape.clone())?;
	emitter.emit(
		vec![diagonal, value.id],
		vec![updates],
		PrimitiveKind::Elementwise(Elementwise {
			program: add_program(request)?,
		}),
	)?;
	emitter.emit(
		vec![matrix.id, indices.id, updates],
		vec![updated.id],
		checked_unique_scatter(0),
	)
}

fn emit_argsort(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(request, &["values", "exposure_indices"], &["indices"], &[
		"elements",
		"exposure_indices_verified_identity",
	])?;
	let elements = positive_parameter(request, "elements")?;
	emit_sorted_prefix(request, emitter, SortedPrefix {
		elements,
		prefix: elements,
		axis: 0,
		prefix_name: "exposure_indices",
		output_name: "indices",
		verification_fact: "exposure_indices_verified_identity",
	})
}

fn emit_partial_argsort(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(request, &["values", "prefix_indices"], &["indices"], &[
		"elements",
		"prefix",
		"prefix_indices_verified",
	])?;
	let elements = positive_parameter(request, "elements")?;
	let prefix = positive_parameter(request, "prefix")?;
	match prefix == elements {
		true => {
			emit_sorted_prefix(request, emitter, SortedPrefix {
				elements,
				prefix,
				axis: 0,
				prefix_name: "prefix_indices",
				output_name: "indices",
				verification_fact: "prefix_indices_verified",
			})
		}
		false => {
			Err(operation_error(
				request.descriptor.id,
				OperationErrorKind::UnsupportedConcreteShape,
				"legacy gpu_partial_argsort writes the complete sorted index vector; a shorter prefix needs a distinct source operation",
			))
		}
	}
}

fn emit_segment_sum(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(request, &["values", "segment_ids"], &["sums"], &[
		"elements",
		"segments",
		"maximum_segment_length",
		"tree_lanes",
	])?;
	let values = input(request, "values")?;
	let segment_ids = input(request, "segment_ids")?;
	let sums = output(request, "sums")?;
	let elements = positive_parameter(request, "elements")?;
	let segments = positive_parameter(request, "segments")?;
	let maximum_segment_length = positive_parameter(request, "maximum_segment_length")?;
	if maximum_segment_length > elements {
		return Err(request_error(
			request,
			"maximum_segment_length exceeds the input element count",
		));
	}
	let grouped_elements = checked_product(
		request,
		segments,
		maximum_segment_length,
		"segmented reduction workspace element count",
	)?;
	require_i32_indexable(request, elements, "segmented reduction input element count")?;
	require_i32_indexable(request, segments, "segmented reduction segment count")?;
	require_i32_indexable(
		request,
		grouped_elements,
		"segmented reduction packed workspace element count",
	)?;
	let tree_lanes = super::prepared_tree_lanes(request)?;
	require_dtype(request, values, DType::F32, "values")?;
	require_dtype(request, segment_ids, DType::I32, "segment_ids")?;
	require_dtype(request, sums, DType::F32, "sums")?;
	require_shape(request, values, &[elements], "values")?;
	require_shape(request, segment_ids, &[elements], "segment_ids")?;
	require_shape(request, sums, &[segments], "sums")?;

	let sorted_ids = emitter.intermediate(DType::I32, segment_ids.shape.clone())?;
	let sorted_original_indices = emitter.intermediate(DType::I32, segment_ids.shape.clone())?;
	emitter.emit(
		vec![segment_ids.id],
		vec![sorted_ids, sorted_original_indices],
		ascending_stable_sort(0),
	)?;

	let positions = emitter.intermediate(DType::I32, segment_ids.shape.clone())?;
	let previous_positions = emitter.intermediate(DType::I32, segment_ids.shape.clone())?;
	let previous_ids = emitter.intermediate(DType::I32, segment_ids.shape.clone())?;
	let boundary_positions = emitter.intermediate(DType::I32, segment_ids.shape.clone())?;
	let segment_starts = emitter.intermediate(DType::I32, segment_ids.shape.clone())?;
	emitter.emit_stage([
		KernelEmission {
			inputs: Vec::new(),
			outputs: vec![positions],
			kind: PrimitiveKind::IndexMap(IndexMap {
				start: 0,
				element_step: 1,
				iteration_step: 0,
				modulus: None,
			}),
		},
		KernelEmission {
			inputs: Vec::new(),
			outputs: vec![previous_positions],
			kind: PrimitiveKind::IndexMap(IndexMap {
				start: -1,
				element_step: 1,
				iteration_step: 0,
				modulus: None,
			}),
		},
		KernelEmission {
			inputs: vec![sorted_ids, previous_positions],
			outputs: vec![previous_ids],
			kind: PrimitiveKind::Gather(Gather {
				axis: 0,
				bounds: IndexBounds::Clamp,
			}),
		},
		KernelEmission {
			inputs: vec![sorted_ids, previous_ids, positions],
			outputs: vec![boundary_positions],
			kind: PrimitiveKind::Elementwise(Elementwise {
				program: segment_boundary_program(request)?,
			}),
		},
		KernelEmission {
			inputs: vec![boundary_positions],
			outputs: vec![segment_starts],
			kind: PrimitiveKind::Scan(Scan {
				operator: ReduceOperator::Maximum,
				axis: 0,
				mode: ScanMode::Inclusive,
				reverse: false,
				tree_lanes,
			}),
		},
	])?;

	let sorted_values = emitter.intermediate(DType::F32, values.shape.clone())?;
	let destinations = emitter.intermediate(DType::I32, segment_ids.shape.clone())?;
	let packed_positions_shape = materialized_shape(request, &[grouped_elements])?;
	let packed_positions = emitter.intermediate(DType::I32, packed_positions_shape.clone())?;
	let zero_base = emitter.intermediate(DType::F32, packed_positions_shape.clone())?;
	let packed = emitter.intermediate(DType::F32, packed_positions_shape)?;
	let matrix_shape = materialized_shape(request, &[segments, maximum_segment_length])?;
	let matrix_positions = emitter.intermediate(DType::I32, matrix_shape.clone())?;
	let grouped = emitter.intermediate(DType::F32, matrix_shape)?;
	let reduction_axes =
		AxisSet::new(vec![1]).map_err(|error| language_error(request.descriptor.id, error.to_string()))?;
	emitter.emit_stage([
		KernelEmission {
			inputs: vec![values.id, sorted_original_indices],
			outputs: vec![sorted_values],
			kind: checked_gather(0),
		},
		KernelEmission {
			inputs: vec![sorted_ids, positions, segment_starts],
			outputs: vec![destinations],
			kind: PrimitiveKind::Elementwise(Elementwise {
				program: segment_destination_program(request, segments, maximum_segment_length)?,
			}),
		},
		KernelEmission {
			inputs: Vec::new(),
			outputs: vec![packed_positions],
			kind: PrimitiveKind::IndexMap(IndexMap {
				start: 0,
				element_step: 1,
				iteration_step: 0,
				modulus: None,
			}),
		},
		KernelEmission {
			inputs: vec![packed_positions],
			outputs: vec![zero_base],
			kind: PrimitiveKind::Elementwise(Elementwise {
				program: zero_from_i32_program(request)?,
			}),
		},
		KernelEmission {
			inputs: vec![zero_base, destinations, sorted_values],
			outputs: vec![packed],
			kind: checked_unique_scatter(0),
		},
		KernelEmission {
			inputs: Vec::new(),
			outputs: vec![matrix_positions],
			kind: PrimitiveKind::IndexMap(IndexMap {
				start: 0,
				element_step: 1,
				iteration_step: 0,
				modulus: None,
			}),
		},
		KernelEmission {
			inputs: vec![packed, matrix_positions],
			outputs: vec![grouped],
			kind: checked_gather(0),
		},
		KernelEmission {
			inputs: vec![grouped],
			outputs: vec![sums.id],
			kind: PrimitiveKind::Reduce(Reduce {
				operator: ReduceOperator::Sum,
				axes: reduction_axes,
				keep_dimensions: false,
				result: ReduceResult::Value,
				tree_lanes,
			}),
		},
	])
}

fn emit_topk_per_row(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(request, &["values", "prefix_indices"], &["indices"], &[
		"rows",
		"columns",
		"k",
		"prefix_indices_verified",
	])?;
	let values = input(request, "values")?;
	let prefix_indices = input(request, "prefix_indices")?;
	let indices = output(request, "indices")?;
	let rows = positive_parameter(request, "rows")?;
	let columns = positive_parameter(request, "columns")?;
	let k = positive_parameter(request, "k")?;
	if k > columns {
		return Err(request_error(request, "k must not exceed columns"));
	}
	require_i32_indexable(request, columns, "column count")?;
	require_true(request, "prefix_indices_verified")?;
	require_dtype(request, values, DType::F32, "values")?;
	require_dtype(request, prefix_indices, DType::I32, "prefix_indices")?;
	require_dtype(request, indices, DType::I32, "indices")?;
	require_shape(request, values, &[rows, columns], "values")?;
	require_shape(request, prefix_indices, &[k], "prefix_indices")?;
	require_shape(request, indices, &[rows, k], "indices")?;

	let sorted_values = emitter.intermediate(DType::F32, values.shape.clone())?;
	let sorted_indices = emitter.intermediate(DType::I32, values.shape.clone())?;
	emitter.emit(
		vec![values.id],
		vec![sorted_values, sorted_indices],
		ascending_stable_sort(1),
	)?;
	emitter.emit(
		vec![sorted_indices, prefix_indices.id],
		vec![indices.id],
		checked_gather(1),
	)
}

fn emit_sorted_prefix(
	request: &MaterializationRequest<'_>,
	emitter: &mut Emitter<'_>,
	specification: SortedPrefix<'_>,
) -> OperationResult<()> {
	let values = input(request, "values")?;
	let prefix_indices = input(request, specification.prefix_name)?;
	let indices = output(request, specification.output_name)?;
	require_i32_indexable(request, specification.elements, "sort extent")?;
	match specification.prefix <= specification.elements {
		true => Ok(()),
		false => {
			Err(request_error(
				request,
				"sorted prefix exceeds the input extent",
			))
		}
	}?;
	require_true(request, specification.verification_fact)?;
	require_dtype(request, values, DType::F32, "values")?;
	require_dtype(
		request,
		prefix_indices,
		DType::I32,
		specification.prefix_name,
	)?;
	require_dtype(request, indices, DType::I32, specification.output_name)?;
	require_shape(request, values, &[specification.elements], "values")?;
	require_shape(
		request,
		prefix_indices,
		&[specification.prefix],
		specification.prefix_name,
	)?;
	require_shape(
		request,
		indices,
		&[specification.prefix],
		specification.output_name,
	)?;
	let sorted_values = emitter.intermediate(DType::F32, values.shape.clone())?;
	let sorted_indices = emitter.intermediate(DType::I32, values.shape.clone())?;
	emitter.emit(
		vec![values.id],
		vec![sorted_values, sorted_indices],
		ascending_stable_sort(specification.axis),
	)?;
	emitter.emit(
		vec![sorted_indices, prefix_indices.id],
		vec![indices.id],
		checked_gather(specification.axis),
	)
}

fn emit_uniform_edges_single_row(
	request: &MaterializationRequest<'_>,
	emitter: &mut Emitter<'_>,
) -> OperationResult<()> {
	require_exact_abi(request, &["feature_values", "bin_indices"], &["edges"], &[
		"rows",
		"columns",
		"bins",
		"bin_indices_verified",
	])?;
	let values = input(request, "feature_values")?;
	let bin_indices = input(request, "bin_indices")?;
	let edges = output(request, "edges")?;
	let rows = positive_parameter(request, "rows")?;
	let columns = positive_parameter(request, "columns")?;
	let bins = positive_parameter(request, "bins")?;
	match rows {
		1 => Ok(()),
		other_rows => {
			Err(operation_error(
				request.descriptor.id,
				OperationErrorKind::UnsupportedConcreteShape,
				format!(
					"gpu_bin_edges_uniform needs a reduction stage for {other_rows} rows; the normative recipe currently contains only Elementwise"
				),
			))
		}
	}?;
	let edge_count = bins.checked_add(1).ok_or_else(|| {
		operation_error(
			request.descriptor.id,
			OperationErrorKind::WorkspaceArithmeticOverflow,
			"uniform edge count overflowed u64",
		)
	})?;
	let bins_f32 = exact_f32_extent(request, bins, "bin count")?;
	require_true(request, "bin_indices_verified")?;
	require_dtype(request, values, DType::F32, "feature_values")?;
	require_dtype(request, bin_indices, DType::F32, "bin_indices")?;
	require_dtype(request, edges, DType::F32, "edges")?;
	require_shape(request, values, &[columns, 1], "feature_values")?;
	require_shape(request, bin_indices, &[1, edge_count], "bin_indices")?;
	require_shape(request, edges, &[columns, edge_count], "edges")?;
	emitter.emit(
		vec![values.id, bin_indices.id],
		vec![edges.id],
		PrimitiveKind::Elementwise(Elementwise {
			program: uniform_single_row_program(request, bins_f32)?,
		}),
	)
}

fn emit_row_concatenation(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(
		request,
		&[
			"left",
			"right",
			"left_indices",
			"right_indices",
			"select_left",
		],
		&["concatenated"],
		&[
			"rows",
			"left_columns",
			"right_columns",
			"concatenation_tables_verified",
		],
	)?;
	let rows = positive_parameter(request, "rows")?;
	let left_columns = positive_parameter(request, "left_columns")?;
	let right_columns = positive_parameter(request, "right_columns")?;
	let left_elements = checked_product(request, rows, left_columns, "left matrix element count")?;
	let right_elements = checked_product(request, rows, right_columns, "right matrix element count")?;
	let total_columns = left_columns.checked_add(right_columns).ok_or_else(|| {
		operation_error(
			request.descriptor.id,
			OperationErrorKind::WorkspaceArithmeticOverflow,
			"concatenated column count overflowed u64",
		)
	})?;
	let total = checked_product(request, rows, total_columns, "concatenated element count")?;
	emit_concatenation(
		request,
		emitter,
		left_elements,
		right_elements,
		total,
		"concatenation_tables_verified",
	)
}

fn emit_vector_concatenation(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(
		request,
		&[
			"left",
			"right",
			"left_indices",
			"right_indices",
			"select_left",
		],
		&["concatenated"],
		&[
			"left_elements",
			"right_elements",
			"concatenation_tables_verified",
		],
	)?;
	let left_elements = positive_parameter(request, "left_elements")?;
	let right_elements = positive_parameter(request, "right_elements")?;
	let total = left_elements.checked_add(right_elements).ok_or_else(|| {
		operation_error(
			request.descriptor.id,
			OperationErrorKind::WorkspaceArithmeticOverflow,
			"vector concatenation element count overflowed u64",
		)
	})?;
	emit_concatenation(
		request,
		emitter,
		left_elements,
		right_elements,
		total,
		"concatenation_tables_verified",
	)
}

fn emit_concatenation(
	request: &MaterializationRequest<'_>,
	emitter: &mut Emitter<'_>,
	left_elements: u64,
	right_elements: u64,
	total: u64,
	verification_fact: &str,
) -> OperationResult<()> {
	let left = input(request, "left")?;
	let right = input(request, "right")?;
	let left_indices = input(request, "left_indices")?;
	let right_indices = input(request, "right_indices")?;
	let select_left = input(request, "select_left")?;
	let concatenated = output(request, "concatenated")?;
	require_i32_indexable(request, left_elements, "left input element count")?;
	require_i32_indexable(request, right_elements, "right input element count")?;
	require_true(request, verification_fact)?;
	require_dtype(request, left_indices, DType::I32, "left_indices")?;
	require_dtype(request, right_indices, DType::I32, "right_indices")?;
	require_dtype(request, select_left, DType::I32, "select_left")?;
	require_dtype(request, right, left.dtype, "right")?;
	require_dtype(request, concatenated, left.dtype, "concatenated")?;
	require_shape(request, left, &[left_elements], "left")?;
	require_shape(request, right, &[right_elements], "right")?;
	require_shape(request, left_indices, &[total], "left_indices")?;
	require_shape(request, right_indices, &[total], "right_indices")?;
	require_shape(request, select_left, &[total], "select_left")?;
	require_shape(request, concatenated, &[total], "concatenated")?;

	let result_shape = concatenated.shape.clone();
	let gathered_left = emitter.intermediate(left.dtype, result_shape.clone())?;
	let gathered_right = emitter.intermediate(right.dtype, result_shape)?;
	emitter.emit_stage([
		KernelEmission {
			inputs: vec![left.id, left_indices.id],
			outputs: vec![gathered_left],
			kind: checked_gather(0),
		},
		KernelEmission {
			inputs: vec![right.id, right_indices.id],
			outputs: vec![gathered_right],
			kind: checked_gather(0),
		},
	])?;
	emitter.emit(
		vec![select_left.id, gathered_left, gathered_right],
		vec![concatenated.id],
		PrimitiveKind::Elementwise(Elementwise {
			program: select_program(request, left.dtype)?,
		}),
	)
}

fn emit_one_hot(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(
		request,
		&["labels", "destination_indices", "output_base"],
		&["encoded"],
		&["rows", "classes", "destination_indices_verified_unique"],
	)?;
	let labels = input(request, "labels")?;
	let destinations = input(request, "destination_indices")?;
	let base = input(request, "output_base")?;
	let encoded = output(request, "encoded")?;
	let rows = positive_parameter(request, "rows")?;
	let classes = positive_parameter(request, "classes")?;
	let elements = checked_product(request, rows, classes, "one-hot element count")?;
	require_i32_indexable(request, elements, "one-hot element count")?;
	let classes_i32 = i32::try_from(classes).map_err(|conversion| request_error(request, conversion.to_string()))?;
	require_true(request, "destination_indices_verified_unique")?;
	require_dtype(request, labels, DType::I32, "labels")?;
	require_dtype(request, destinations, DType::I32, "destination_indices")?;
	require_dtype(request, base, DType::F32, "output_base")?;
	require_dtype(request, encoded, DType::F32, "encoded")?;
	require_shape(request, labels, &[rows], "labels")?;
	require_shape(request, destinations, &[rows], "destination_indices")?;
	require_shape(request, base, &[elements], "output_base")?;
	require_shape(request, encoded, &[elements], "encoded")?;

	let cleared = emitter.intermediate(DType::F32, base.shape.clone())?;
	let updates = emitter.intermediate(DType::F32, labels.shape.clone())?;
	emitter.emit_stage([
		KernelEmission {
			inputs: vec![base.id],
			outputs: vec![cleared],
			kind: PrimitiveKind::Elementwise(Elementwise {
				program: clear_program(request)?,
			}),
		},
		KernelEmission {
			inputs: vec![labels.id],
			outputs: vec![updates],
			kind: PrimitiveKind::Elementwise(Elementwise {
				program: checked_one_program(request, classes_i32)?,
			}),
		},
	])?;
	emitter.emit(
		vec![cleared, destinations.id, updates],
		vec![encoded.id],
		checked_unique_scatter(0),
	)
}

fn emit_dense_upper_triangle(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(
		request,
		&["factor", "factor_indices", "upper_mask", "zero"],
		&["upper_triangle"],
		&[
			"factor_rows",
			"dimension",
			"factor_indices_verified",
			"upper_mask_verified",
			"zero_verified",
		],
	)?;
	let factor = input(request, "factor")?;
	let indices = input(request, "factor_indices")?;
	let mask = input(request, "upper_mask")?;
	let zero = input(request, "zero")?;
	let upper = output(request, "upper_triangle")?;
	let factor_rows = positive_parameter(request, "factor_rows")?;
	let dimension = positive_parameter(request, "dimension")?;
	match factor_rows >= dimension {
		true => Ok(()),
		false => {
			Err(request_error(
				request,
				"factor_rows must be at least dimension",
			))
		}
	}?;
	let factor_elements = checked_product(
		request,
		factor_rows,
		dimension,
		"upper-triangle factor element count",
	)?;
	let output_elements = checked_product(
		request,
		dimension,
		dimension,
		"dense upper-triangle element count",
	)?;
	require_i32_indexable(request, factor_elements, "factor element count")?;
	require_true(request, "factor_indices_verified")?;
	require_true(request, "upper_mask_verified")?;
	require_true(request, "zero_verified")?;
	require_dtype(request, factor, DType::F32, "factor")?;
	require_dtype(request, indices, DType::I32, "factor_indices")?;
	require_dtype(request, mask, DType::I32, "upper_mask")?;
	require_dtype(request, zero, DType::F32, "zero")?;
	require_dtype(request, upper, DType::F32, "upper_triangle")?;
	require_shape(request, factor, &[factor_elements], "factor")?;
	require_shape(request, indices, &[output_elements], "factor_indices")?;
	require_shape(request, mask, &[output_elements], "upper_mask")?;
	require_shape(request, zero, &[1], "zero")?;
	require_shape(request, upper, &[output_elements], "upper_triangle")?;

	let gathered = emitter.intermediate(DType::F32, upper.shape.clone())?;
	emitter.emit(
		vec![factor.id, indices.id],
		vec![gathered],
		checked_gather(0),
	)?;
	emitter.emit(
		vec![mask.id, gathered, zero.id],
		vec![upper.id],
		PrimitiveKind::Elementwise(Elementwise {
			program: select_program(request, DType::F32)?,
		}),
	)
}

fn emit_column_slice(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(request, &["values", "slice_indices"], &["sliced"], &[
		"rows",
		"columns",
		"start",
		"count",
		"slice_indices_verified",
	])?;
	let rows = positive_parameter(request, "rows")?;
	let columns = positive_parameter(request, "columns")?;
	let start = prepared_u64(request.descriptor.id, request.parameters, "start")?;
	let count = positive_parameter(request, "count")?;
	require_range(request, start, count, columns, "column slice")?;
	let source = checked_product(request, rows, columns, "column-slice source element count")?;
	let result = checked_product(request, rows, count, "column-slice result element count")?;
	emit_flat_gather_map(request, emitter, source, result, "slice_indices_verified")
}

fn emit_leading_slice(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(request, &["values", "slice_indices"], &["sliced"], &[
		"rows",
		"source_columns",
		"take",
		"slice_indices_verified",
	])?;
	let rows = positive_parameter(request, "rows")?;
	let columns = positive_parameter(request, "source_columns")?;
	let take = positive_parameter(request, "take")?;
	require_range(request, 0, take, columns, "leading column slice")?;
	let source = checked_product(request, rows, columns, "leading-slice source element count")?;
	let result = checked_product(request, rows, take, "leading-slice result element count")?;
	emit_flat_gather_map(request, emitter, source, result, "slice_indices_verified")
}

fn emit_row_slice(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(request, &["values", "slice_indices"], &["sliced"], &[
		"rows",
		"columns",
		"start",
		"count",
		"slice_indices_verified",
	])?;
	let rows = positive_parameter(request, "rows")?;
	let columns = positive_parameter(request, "columns")?;
	let start = prepared_u64(request.descriptor.id, request.parameters, "start")?;
	let count = positive_parameter(request, "count")?;
	require_range(request, start, count, rows, "row slice")?;
	let source = checked_product(request, rows, columns, "row-slice source element count")?;
	let result = checked_product(request, count, columns, "row-slice result element count")?;
	emit_flat_gather_map(request, emitter, source, result, "slice_indices_verified")
}

fn emit_transpose(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(
		request,
		&["values", "transpose_indices"],
		&["transposed"],
		&["rows", "columns", "transpose_indices_verified"],
	)?;
	let rows = positive_parameter(request, "rows")?;
	let columns = positive_parameter(request, "columns")?;
	let elements = checked_product(request, rows, columns, "transpose element count")?;
	emit_flat_gather_map(
		request,
		emitter,
		elements,
		elements,
		"transpose_indices_verified",
	)
}

fn emit_flat_gather_map(
	request: &MaterializationRequest<'_>,
	emitter: &mut Emitter<'_>,
	source_elements: u64,
	result_elements: u64,
	verification_fact: &str,
) -> OperationResult<()> {
	let values = input(request, "values")?;
	let indices_name = match request.descriptor.symbol {
		"gpu_transpose" => "transpose_indices",
		"gpu_slice_cols" | "gpu_slice_lead_into" | "gpu_slice_rows" => "slice_indices",
		symbol => {
			return Err(request_error(
				request,
				format!("flat gather table name is unspecified for {symbol}"),
			));
		}
	};
	let output_name = match request.descriptor.symbol {
		"gpu_transpose" => "transposed",
		"gpu_slice_cols" | "gpu_slice_lead_into" | "gpu_slice_rows" => "sliced",
		symbol => {
			return Err(request_error(
				request,
				format!("flat gather output name is unspecified for {symbol}"),
			));
		}
	};
	let indices = input(request, indices_name)?;
	let result = output(request, output_name)?;
	require_i32_indexable(request, source_elements, "gather source element count")?;
	require_true(request, verification_fact)?;
	require_dtype(request, values, DType::F32, "values")?;
	require_dtype(request, indices, DType::I32, indices_name)?;
	require_dtype(request, result, DType::F32, output_name)?;
	require_shape(request, values, &[source_elements], "values")?;
	require_shape(request, indices, &[result_elements], indices_name)?;
	require_shape(request, result, &[result_elements], output_name)?;
	let gathered = emitter.intermediate(DType::F32, result.shape.clone())?;
	emitter.emit(
		vec![values.id, indices.id],
		vec![gathered],
		checked_gather(0),
	)?;
	emitter.emit(
		vec![gathered],
		vec![result.id],
		PrimitiveKind::Elementwise(Elementwise {
			program: identity_program(request.descriptor.id, DType::F32)?,
		}),
	)
}

fn emit_triangular_mask(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(
		request,
		&["row_indices", "column_indices", "fill_value"],
		&["mask"],
		&["dimension", "coordinate_tables_verified"],
	)?;
	let rows = input(request, "row_indices")?;
	let columns = input(request, "column_indices")?;
	let fill = input(request, "fill_value")?;
	let mask = output(request, "mask")?;
	let dimension = positive_parameter(request, "dimension")?;
	let elements = checked_product(
		request,
		dimension,
		dimension,
		"triangular mask element count",
	)?;
	require_i32_indexable(request, dimension, "triangular mask dimension")?;
	require_true(request, "coordinate_tables_verified")?;
	require_dtype(request, rows, DType::I32, "row_indices")?;
	require_dtype(request, columns, DType::I32, "column_indices")?;
	require_dtype(request, fill, DType::F32, "fill_value")?;
	require_dtype(request, mask, DType::F32, "mask")?;
	require_shape(request, rows, &[dimension, dimension], "row_indices")?;
	require_shape(request, columns, &[dimension, dimension], "column_indices")?;
	require_shape(request, fill, &[1], "fill_value")?;
	require_shape(request, mask, &[dimension, dimension], "mask")?;
	match elements {
		0 => {
			Err(request_error(
				request,
				"triangular mask dimension must be positive",
			))
		}
		positive_elements => {
			let confirmed_elements = positive_elements;
			match confirmed_elements == mask.shape.elements() {
				true => {
					emitter.emit(
						vec![rows.id, columns.id, fill.id],
						vec![mask.id],
						PrimitiveKind::Elementwise(Elementwise {
							program: triangular_mask_program(request)?,
						}),
					)
				}
				false => {
					Err(request_error(
						request,
						"triangular mask element count is inconsistent",
					))
				}
			}
		}
	}
}

fn checked_gather(axis: usize) -> PrimitiveKind {
	PrimitiveKind::Gather(Gather {
		axis,
		bounds: IndexBounds::Reject,
	})
}

fn checked_unique_scatter(axis: usize) -> PrimitiveKind {
	PrimitiveKind::Scatter(Scatter {
		axis,
		bounds: IndexBounds::Reject,
		conflict: ScatterConflict::UniqueIndices,
	})
}

fn ascending_stable_sort(axis: usize) -> PrimitiveKind {
	PrimitiveKind::Sort(Sort {
		axis,
		direction: SortDirection::Ascending,
		stable: true,
		emit_indices: true,
	})
}

fn positive_parameter(request: &MaterializationRequest<'_>, name: &str) -> OperationResult<u64> {
	match prepared_u64(request.descriptor.id, request.parameters, name)? {
		0 => {
			Err(request_error(
				request,
				format!("{name} must be greater than zero"),
			))
		}
		value => Ok(value),
	}
}

fn checked_product(request: &MaterializationRequest<'_>, left: u64, right: u64, label: &str) -> OperationResult<u64> {
	left.checked_mul(right).ok_or_else(|| {
		operation_error(
			request.descriptor.id,
			OperationErrorKind::WorkspaceArithmeticOverflow,
			format!("{label} overflowed u64"),
		)
	})
}

fn materialized_shape(request: &MaterializationRequest<'_>, extents: &[u64]) -> OperationResult<Shape> {
	Shape::new(extents.to_vec()).map_err(|error| language_error(request.descriptor.id, error.to_string()))
}

fn segment_boundary_program(request: &MaterializationRequest<'_>) -> OperationResult<recipe_core::ScalarProgram> {
	let operation = request.descriptor.id;
	let mut builder = scalar_builder(operation)?;
	let current = scalar_input(operation, &mut builder, DType::I32)?;
	let previous = scalar_input(operation, &mut builder, DType::I32)?;
	let position = scalar_input(operation, &mut builder, DType::I32)?;
	let zero = builder
		.i32(0)
		.map_err(|error| language_error(operation, error.to_string()))?;
	let first = scalar_binary(operation, &mut builder, ScalarOpcode::Equal, position, zero)?;
	let changed = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::NotEqual,
		current,
		previous,
	)?;
	let boundary = scalar_binary(operation, &mut builder, ScalarOpcode::BitOr, first, changed)?;
	let start = scalar_ternary(
		operation,
		&mut builder,
		ScalarOpcode::Select,
		boundary,
		position,
		zero,
	)?;
	scalar_finish(operation, builder, &[start])
}

fn segment_destination_program(
	request: &MaterializationRequest<'_>,
	segments: u64,
	maximum_segment_length: u64,
) -> OperationResult<recipe_core::ScalarProgram> {
	let operation = request.descriptor.id;
	let segments = i32::try_from(segments).map_err(|error| request_error(request, error.to_string()))?;
	let maximum_segment_length =
		i32::try_from(maximum_segment_length).map_err(|error| request_error(request, error.to_string()))?;
	let mut builder = scalar_builder(operation)?;
	let segment = scalar_input(operation, &mut builder, DType::I32)?;
	let position = scalar_input(operation, &mut builder, DType::I32)?;
	let start = scalar_input(operation, &mut builder, DType::I32)?;
	let zero = builder
		.i32(0)
		.map_err(|error| language_error(operation, error.to_string()))?;
	let segment_limit = builder
		.i32(segments)
		.map_err(|error| language_error(operation, error.to_string()))?;
	let length_limit = builder
		.i32(maximum_segment_length)
		.map_err(|error| language_error(operation, error.to_string()))?;
	let local = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Subtract,
		position,
		start,
	)?;
	let segment_nonnegative = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::GreaterThanOrEqual,
		segment,
		zero,
	)?;
	let segment_bounded = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::LessThan,
		segment,
		segment_limit,
	)?;
	let local_nonnegative = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::GreaterThanOrEqual,
		local,
		zero,
	)?;
	let local_bounded = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::LessThan,
		local,
		length_limit,
	)?;
	let valid_segment = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::BitAnd,
		segment_nonnegative,
		segment_bounded,
	)?;
	let valid_local = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::BitAnd,
		local_nonnegative,
		local_bounded,
	)?;
	let valid = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::BitAnd,
		valid_segment,
		valid_local,
	)?;
	scalar_unary(operation, &mut builder, ScalarOpcode::Require, valid)?;
	let base = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		segment,
		length_limit,
	)?;
	let destination = scalar_binary(operation, &mut builder, ScalarOpcode::Add, base, local)?;
	scalar_finish(operation, builder, &[destination])
}

fn zero_from_i32_program(request: &MaterializationRequest<'_>) -> OperationResult<recipe_core::ScalarProgram> {
	let operation = request.descriptor.id;
	let mut builder = scalar_builder(operation)?;
	let position = scalar_input(operation, &mut builder, DType::I32)?;
	let consumed = scalar_unary(
		operation,
		&mut builder,
		ScalarOpcode::ConvertI32ToF32,
		position,
	)?;
	let zero = scalar_f32(operation, &mut builder, 0.0)?;
	let always = builder
		.i32(1)
		.map_err(|error| language_error(operation, error.to_string()))?;
	let cleared = scalar_ternary(
		operation,
		&mut builder,
		ScalarOpcode::Select,
		always,
		zero,
		consumed,
	)?;
	scalar_finish(operation, builder, &[cleared])
}

fn require_i32_indexable(request: &MaterializationRequest<'_>, extent: u64, label: &str) -> OperationResult<()> {
	match (extent > 0, extent <= MAX_I32_INDEX) {
		(true, true) => Ok(()),
		(false, true) | (false, false) => {
			Err(operation_error(
				request.descriptor.id,
				OperationErrorKind::UnsupportedConcreteShape,
				format!("{label} must be positive"),
			))
		}
		(true, false) => {
			Err(operation_error(
				request.descriptor.id,
				OperationErrorKind::UnsupportedConcreteShape,
				format!("{label} must fit checked int32 indexes"),
			))
		}
	}
}

fn require_range(
	request: &MaterializationRequest<'_>,
	start: u64,
	count: u64,
	extent: u64,
	label: &str,
) -> OperationResult<()> {
	let end = start
		.checked_add(count)
		.ok_or_else(|| request_error(request, format!("{label} start plus count overflowed u64")))?;
	match end <= extent {
		true => Ok(()),
		false => {
			Err(operation_error(
				request.descriptor.id,
				OperationErrorKind::UnsupportedConcreteShape,
				format!("{label} end {end} exceeds extent {extent}"),
			))
		}
	}
}

fn add_program(request: &MaterializationRequest<'_>) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(request.descriptor.id)?;
	let left = scalar_input(request.descriptor.id, &mut builder, DType::F32)?;
	let right = scalar_input(request.descriptor.id, &mut builder, DType::F32)?;
	let sum = scalar_binary(
		request.descriptor.id,
		&mut builder,
		ScalarOpcode::Add,
		left,
		right,
	)?;
	scalar_finish(request.descriptor.id, builder, &[sum])
}

fn scaled_add_program(request: &MaterializationRequest<'_>) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(request.descriptor.id)?;
	let old = scalar_input(request.descriptor.id, &mut builder, DType::F32)?;
	let column = scalar_input(request.descriptor.id, &mut builder, DType::F32)?;
	let scale = scalar_input(request.descriptor.id, &mut builder, DType::F32)?;
	let scaled = scalar_binary(
		request.descriptor.id,
		&mut builder,
		ScalarOpcode::Multiply,
		column,
		scale,
	)?;
	let updated = scalar_binary(
		request.descriptor.id,
		&mut builder,
		ScalarOpcode::Add,
		old,
		scaled,
	)?;
	scalar_finish(request.descriptor.id, builder, &[updated])
}

fn uniform_single_row_program(
	request: &MaterializationRequest<'_>,
	bins: f32,
) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(request.descriptor.id)?;
	let value = scalar_input(request.descriptor.id, &mut builder, DType::F32)?;
	let bin_index = scalar_input(request.descriptor.id, &mut builder, DType::F32)?;
	let difference = scalar_binary(
		request.descriptor.id,
		&mut builder,
		ScalarOpcode::Subtract,
		value,
		value,
	)?;
	let divisor = scalar_f32(request.descriptor.id, &mut builder, bins)?;
	let width = scalar_binary(
		request.descriptor.id,
		&mut builder,
		ScalarOpcode::Divide,
		difference,
		divisor,
	)?;
	let offset = scalar_binary(
		request.descriptor.id,
		&mut builder,
		ScalarOpcode::Multiply,
		bin_index,
		width,
	)?;
	let edge = scalar_binary(
		request.descriptor.id,
		&mut builder,
		ScalarOpcode::Add,
		value,
		offset,
	)?;
	scalar_finish(request.descriptor.id, builder, &[edge])
}

fn select_program(request: &MaterializationRequest<'_>, dtype: DType) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(request.descriptor.id)?;
	let condition = scalar_input(request.descriptor.id, &mut builder, DType::I32)?;
	let selected = scalar_input(request.descriptor.id, &mut builder, dtype)?;
	let alternate = scalar_input(request.descriptor.id, &mut builder, dtype)?;
	let result = scalar_ternary(
		request.descriptor.id,
		&mut builder,
		ScalarOpcode::Select,
		condition,
		selected,
		alternate,
	)?;
	scalar_finish(request.descriptor.id, builder, &[result])
}

fn clear_program(request: &MaterializationRequest<'_>) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(request.descriptor.id)?;
	let consumed = scalar_input(request.descriptor.id, &mut builder, DType::F32)?;
	let zero = scalar_f32(request.descriptor.id, &mut builder, 0.0)?;
	let always = builder
		.i32(1)
		.map_err(|error| language_error(request.descriptor.id, error.to_string()))?;
	let cleared = scalar_ternary(
		request.descriptor.id,
		&mut builder,
		ScalarOpcode::Select,
		always,
		zero,
		consumed,
	)?;
	scalar_finish(request.descriptor.id, builder, &[cleared])
}

fn checked_one_program(
	request: &MaterializationRequest<'_>,
	classes: i32,
) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(request.descriptor.id)?;
	let label = scalar_input(request.descriptor.id, &mut builder, DType::I32)?;
	let zero = builder
		.i32(0)
		.map_err(|error| language_error(request.descriptor.id, error.to_string()))?;
	let upper = builder
		.i32(classes)
		.map_err(|error| language_error(request.descriptor.id, error.to_string()))?;
	let nonnegative = scalar_binary(
		request.descriptor.id,
		&mut builder,
		ScalarOpcode::GreaterThanOrEqual,
		label,
		zero,
	)?;
	let below_upper = scalar_binary(
		request.descriptor.id,
		&mut builder,
		ScalarOpcode::LessThan,
		label,
		upper,
	)?;
	let valid = scalar_binary(
		request.descriptor.id,
		&mut builder,
		ScalarOpcode::BitAnd,
		nonnegative,
		below_upper,
	)?;
	scalar_unary(
		request.descriptor.id,
		&mut builder,
		ScalarOpcode::Require,
		valid,
	)?;
	let one = scalar_f32(request.descriptor.id, &mut builder, 1.0)?;
	scalar_finish(request.descriptor.id, builder, &[one])
}

fn triangular_mask_program(request: &MaterializationRequest<'_>) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(request.descriptor.id)?;
	let row = scalar_input(request.descriptor.id, &mut builder, DType::I32)?;
	let column = scalar_input(request.descriptor.id, &mut builder, DType::I32)?;
	let fill = scalar_input(request.descriptor.id, &mut builder, DType::F32)?;
	let retained = scalar_binary(
		request.descriptor.id,
		&mut builder,
		ScalarOpcode::LessThanOrEqual,
		column,
		row,
	)?;
	let zero = scalar_f32(request.descriptor.id, &mut builder, 0.0)?;
	let value = scalar_ternary(
		request.descriptor.id,
		&mut builder,
		ScalarOpcode::Select,
		retained,
		zero,
		fill,
	)?;
	scalar_finish(request.descriptor.id, builder, &[value])
}
