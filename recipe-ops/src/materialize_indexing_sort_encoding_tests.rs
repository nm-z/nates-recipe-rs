use std::collections::BTreeSet;
use std::error::Error;

use recipe_core::{ByteCount, DType, KernelTemplateId, ScalarOpcode, ScalarProgram, ValueId};
use recipe_language::{IndexBounds, PrimitiveKind, ScatterConflict, Shape, SortDirection, Tensor};

use crate::{
	IdentityNamespace, MaterializationRequest, MaterializedComposition, NamedTensor, OperationErrorKind,
	PreparedParameter, PreparedParameters, materialize_composition, operation_registry,
	remaining_composition_manifest,
};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

const MATERIALIZED_SYMBOLS: &[&str] = &[
	"gpu_add_col",
	"gpu_add_col_scaled_inplace",
	"gpu_add_diag",
	"gpu_argsort",
	"gpu_bin_edges_uniform",
	"gpu_concat_into",
	"gpu_one_hot",
	"gpu_pack_upper_tri",
	"gpu_partial_argsort",
	"gpu_slice_cols",
	"gpu_slice_lead_into",
	"gpu_slice_rows",
	"gpu_topk_per_row",
	"gpu_transpose",
	"gpu_tril_mask",
	"gpu_vconcat",
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

fn input_tensor(id: u64, dtype: DType, extents: &[u64]) -> TestResult<Tensor> {
	tensor(id, dtype, extents, true, false)
}

fn output_tensor(id: u64, dtype: DType, extents: &[u64]) -> TestResult<Tensor> {
	tensor(id, dtype, extents, false, true)
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

fn assert_fragment(materialized: &MaterializedComposition, stages: usize, nodes: usize) -> TestResult {
	materialized.graph().validate()?;
	assert_eq!(materialized.stages().len(), stages);
	assert_eq!(materialized.graph().nodes.len(), nodes);
	Ok(())
}

fn assert_checked_gather(kind: &PrimitiveKind) -> TestResult {
	match kind {
		PrimitiveKind::Gather(gather) => {
			assert_eq!(gather.bounds, IndexBounds::Reject);
			Ok(())
		}
		other => Err(Box::<dyn Error>::from(format!(
			"expected checked gather, received {other:?}"
		))),
	}
}

fn assert_unique_scatter(kind: &PrimitiveKind) -> TestResult {
	match kind {
		PrimitiveKind::Scatter(scatter) => {
			assert_eq!(scatter.bounds, IndexBounds::Reject);
			assert_eq!(scatter.conflict, ScatterConflict::UniqueIndices);
			Ok(())
		}
		other => Err(Box::<dyn Error>::from(format!(
			"expected unique scatter, received {other:?}"
		))),
	}
}

fn assert_stable_sort(kind: &PrimitiveKind, axis: usize) -> TestResult {
	match kind {
		PrimitiveKind::Sort(sort) => {
			assert_eq!(sort.axis, axis);
			assert_eq!(sort.direction, SortDirection::Ascending);
			assert!(sort.stable);
			assert!(sort.emit_indices);
			Ok(())
		}
		other => Err(Box::<dyn Error>::from(format!(
			"expected stable ascending sort, received {other:?}"
		))),
	}
}

fn elementwise_program(kind: &PrimitiveKind) -> TestResult<&ScalarProgram> {
	match kind {
		PrimitiveKind::Elementwise(elementwise) => Ok(&elementwise.program),
		other => Err(Box::<dyn Error>::from(format!(
			"expected elementwise program, received {other:?}"
		))),
	}
}

#[test]
fn owns_exactly_sixteen_source_qualified_descriptors() -> TestResult {
	let remaining = remaining_composition_manifest()
		.into_iter()
		.map(|entry| (entry.symbol(), entry.source()))
		.collect::<BTreeSet<_>>();
	for symbol in MATERIALIZED_SYMBOLS {
		let descriptor = operation_registry().resolve_unique(symbol)?;
		assert!(!remaining.contains(&(descriptor.symbol, descriptor.source)));
	}

	let matrix = input_tensor(1, DType::F32, &[4])?;
	let column = input_tensor(2, DType::F32, &[2])?;
	let indices = input_tensor(3, DType::I32, &[2])?;
	let updated = output_tensor(4, DType::F32, &[4])?;
	let inputs = [
		NamedTensor::new("matrix", &matrix),
		NamedTensor::new("column", &column),
		NamedTensor::new("column_indices", &indices),
	];
	let outputs = [NamedTensor::new("updated_matrix", &updated)];
	let prepared = parameters(&[
		("rows", PreparedParameter::U64(2)),
		("columns", PreparedParameter::U64(2)),
		("column_index", PreparedParameter::U64(1)),
		(
			"column_indices_verified_unique",
			PreparedParameter::Bool(true),
		),
	]);
	let mut wrong_source = operation_registry().resolve_unique("gpu_add_col")?;
	wrong_source.source = "gpu-core/src/kernels.rs:6840";
	let error = materialize_composition(MaterializationRequest::new(
		wrong_source,
		&inputs,
		&outputs,
		"matrix",
		&prepared,
		namespace(10_000),
		ByteCount::new(16),
	))
	.expect_err("symbol-only dispatch accepted a mismatched source");
	assert_eq!(error.kind, OperationErrorKind::MissingConcreteFormula);
	Ok(())
}

#[test]
fn materializes_column_and_diagonal_updates() -> TestResult {
	let matrix = input_tensor(1, DType::F32, &[12])?;
	let column = input_tensor(2, DType::F32, &[3])?;
	let column_indices = input_tensor(3, DType::I32, &[3])?;
	let updated = output_tensor(4, DType::F32, &[12])?;
	let inputs = [
		NamedTensor::new("matrix", &matrix),
		NamedTensor::new("column", &column),
		NamedTensor::new("column_indices", &column_indices),
	];
	let outputs = [NamedTensor::new("updated_matrix", &updated)];
	let prepared = parameters(&[
		("rows", PreparedParameter::U64(3)),
		("columns", PreparedParameter::U64(4)),
		("column_index", PreparedParameter::U64(2)),
		(
			"column_indices_verified_unique",
			PreparedParameter::Bool(true),
		),
	]);
	let add = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_add_col")?,
		&inputs,
		&outputs,
		"matrix",
		&prepared,
		namespace(20_000),
		ByteCount::new(24),
	))?;
	assert_fragment(&add, 3, 3)?;
	assert_eq!(add.workspace().bytes(), ByteCount::new(24));
	assert_checked_gather(&add.graph().nodes[0].kernel.kind)?;
	assert_unique_scatter(&add.graph().nodes[2].kernel.kind)?;

	let scale = input_tensor(5, DType::F32, &[1])?;
	let scaled_inputs = [
		NamedTensor::new("matrix", &matrix),
		NamedTensor::new("column", &column),
		NamedTensor::new("scale", &scale),
		NamedTensor::new("column_indices", &column_indices),
	];
	let scaled = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_add_col_scaled_inplace")?,
		&scaled_inputs,
		&outputs,
		"matrix",
		&prepared,
		namespace(22_000),
		ByteCount::new(24),
	))?;
	assert_fragment(&scaled, 3, 3)?;
	let scaled_program = elementwise_program(&scaled.graph().nodes[1].kernel.kind)?;
	assert!(
		scaled_program
			.instructions
			.iter()
			.any(|instruction| instruction.opcode == ScalarOpcode::Multiply)
	);
	assert_unique_scatter(&scaled.graph().nodes[2].kernel.kind)?;

	let diagonal_matrix = input_tensor(11, DType::F32, &[9])?;
	let diagonal_value = input_tensor(12, DType::F32, &[1])?;
	let diagonal_indices = input_tensor(13, DType::I32, &[3])?;
	let diagonal_updated = output_tensor(14, DType::F32, &[9])?;
	let diagonal_inputs = [
		NamedTensor::new("matrix", &diagonal_matrix),
		NamedTensor::new("diagonal_value", &diagonal_value),
		NamedTensor::new("diagonal_indices", &diagonal_indices),
	];
	let diagonal_outputs = [NamedTensor::new("updated_matrix", &diagonal_updated)];
	let diagonal_prepared = parameters(&[
		("dimension", PreparedParameter::U64(3)),
		(
			"diagonal_indices_verified_unique",
			PreparedParameter::Bool(true),
		),
	]);
	let diagonal = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_add_diag")?,
		&diagonal_inputs,
		&diagonal_outputs,
		"matrix",
		&diagonal_prepared,
		namespace(24_000),
		ByteCount::new(24),
	))?;
	assert_fragment(&diagonal, 3, 3)?;
	assert_checked_gather(&diagonal.graph().nodes[0].kernel.kind)?;
	assert_unique_scatter(&diagonal.graph().nodes[2].kernel.kind)?;
	Ok(())
}

#[test]
fn materializes_full_and_rowwise_index_sorts() -> TestResult {
	let values = input_tensor(1, DType::F32, &[5])?;
	let exposure = input_tensor(2, DType::I32, &[5])?;
	let indices = output_tensor(3, DType::I32, &[5])?;
	let argsort_inputs = [
		NamedTensor::new("values", &values),
		NamedTensor::new("exposure_indices", &exposure),
	];
	let outputs = [NamedTensor::new("indices", &indices)];
	let argsort_parameters = parameters(&[
		("elements", PreparedParameter::U64(5)),
		(
			"exposure_indices_verified_identity",
			PreparedParameter::Bool(true),
		),
	]);
	let argsort = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_argsort")?,
		&argsort_inputs,
		&outputs,
		"values",
		&argsort_parameters,
		namespace(30_000),
		ByteCount::new(40),
	))?;
	assert_fragment(&argsort, 2, 2)?;
	assert_stable_sort(&argsort.graph().nodes[0].kernel.kind, 0)?;
	assert_checked_gather(&argsort.graph().nodes[1].kernel.kind)?;

	let prefix = input_tensor(4, DType::I32, &[5])?;
	let partial_inputs = [
		NamedTensor::new("values", &values),
		NamedTensor::new("prefix_indices", &prefix),
	];
	let partial_parameters = parameters(&[
		("elements", PreparedParameter::U64(5)),
		("prefix", PreparedParameter::U64(5)),
		("prefix_indices_verified", PreparedParameter::Bool(true)),
	]);
	let partial = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_partial_argsort")?,
		&partial_inputs,
		&outputs,
		"values",
		&partial_parameters,
		namespace(32_000),
		ByteCount::new(40),
	))?;
	assert_fragment(&partial, 2, 2)?;
	assert_stable_sort(&partial.graph().nodes[0].kernel.kind, 0)?;

	let row_values = input_tensor(11, DType::F32, &[2, 6])?;
	let row_prefix = input_tensor(12, DType::I32, &[3])?;
	let row_indices = output_tensor(13, DType::I32, &[2, 3])?;
	let row_inputs = [
		NamedTensor::new("values", &row_values),
		NamedTensor::new("prefix_indices", &row_prefix),
	];
	let row_outputs = [NamedTensor::new("indices", &row_indices)];
	let row_parameters = parameters(&[
		("rows", PreparedParameter::U64(2)),
		("columns", PreparedParameter::U64(6)),
		("k", PreparedParameter::U64(3)),
		("prefix_indices_verified", PreparedParameter::Bool(true)),
	]);
	let topk = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_topk_per_row")?,
		&row_inputs,
		&row_outputs,
		"values",
		&row_parameters,
		namespace(34_000),
		ByteCount::new(96),
	))?;
	assert_fragment(&topk, 2, 2)?;
	assert_stable_sort(&topk.graph().nodes[0].kernel.kind, 1)?;
	assert_checked_gather(&topk.graph().nodes[1].kernel.kind)?;
	Ok(())
}

fn materialize_concatenation(
	symbol: &str,
	parameter_values: &[(&str, PreparedParameter)],
	seed: u64,
) -> TestResult<MaterializedComposition> {
	let left = input_tensor(1, DType::F32, &[4])?;
	let right = input_tensor(2, DType::F32, &[2])?;
	let left_indices = input_tensor(3, DType::I32, &[6])?;
	let right_indices = input_tensor(4, DType::I32, &[6])?;
	let selector = input_tensor(5, DType::I32, &[6])?;
	let result = output_tensor(6, DType::F32, &[6])?;
	let inputs = [
		NamedTensor::new("left", &left),
		NamedTensor::new("right", &right),
		NamedTensor::new("left_indices", &left_indices),
		NamedTensor::new("right_indices", &right_indices),
		NamedTensor::new("select_left", &selector),
	];
	let outputs = [NamedTensor::new("concatenated", &result)];
	let prepared = parameters(parameter_values);
	Ok(materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique(symbol)?,
		&inputs,
		&outputs,
		"left",
		&prepared,
		namespace(seed),
		ByteCount::new(48),
	))?)
}

fn materialize_flat_indexing(
	symbol: &str,
	index_name: &str,
	output_name: &str,
	source_elements: u64,
	result_elements: u64,
	parameter_values: &[(&str, PreparedParameter)],
	seed: u64,
) -> TestResult<MaterializedComposition> {
	let values = input_tensor(1, DType::F32, &[source_elements])?;
	let indexes = input_tensor(2, DType::I32, &[result_elements])?;
	let result = output_tensor(3, DType::F32, &[result_elements])?;
	let inputs = [
		NamedTensor::new("values", &values),
		NamedTensor::new(index_name, &indexes),
	];
	let outputs = [NamedTensor::new(output_name, &result)];
	let prepared = parameters(parameter_values);
	Ok(materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique(symbol)?,
		&inputs,
		&outputs,
		"values",
		&prepared,
		namespace(seed),
		ByteCount::new(result_elements * 4),
	))?)
}

#[test]
fn materializes_concatenations_slices_and_transpose() -> TestResult {
	let row_concat = materialize_concatenation(
		"gpu_concat_into",
		&[
			("rows", PreparedParameter::U64(2)),
			("left_columns", PreparedParameter::U64(2)),
			("right_columns", PreparedParameter::U64(1)),
			(
				"concatenation_tables_verified",
				PreparedParameter::Bool(true),
			),
		],
		40_000,
	)?;
	assert_fragment(&row_concat, 2, 3)?;
	assert_eq!(row_concat.workspace().bytes(), ByteCount::new(48));
	assert_checked_gather(&row_concat.graph().nodes[0].kernel.kind)?;
	assert_checked_gather(&row_concat.graph().nodes[1].kernel.kind)?;

	let vector_concat = materialize_concatenation(
		"gpu_vconcat",
		&[
			("left_elements", PreparedParameter::U64(4)),
			("right_elements", PreparedParameter::U64(2)),
			(
				"concatenation_tables_verified",
				PreparedParameter::Bool(true),
			),
		],
		42_000,
	)?;
	assert_fragment(&vector_concat, 2, 3)?;

	let column_slice = materialize_flat_indexing(
		"gpu_slice_cols",
		"slice_indices",
		"sliced",
		12,
		6,
		&[
			("rows", PreparedParameter::U64(3)),
			("columns", PreparedParameter::U64(4)),
			("start", PreparedParameter::U64(1)),
			("count", PreparedParameter::U64(2)),
			("slice_indices_verified", PreparedParameter::Bool(true)),
		],
		44_000,
	)?;
	assert_fragment(&column_slice, 2, 2)?;
	assert_eq!(column_slice.workspace().bytes(), ByteCount::new(24));

	let leading_slice = materialize_flat_indexing(
		"gpu_slice_lead_into",
		"slice_indices",
		"sliced",
		12,
		6,
		&[
			("rows", PreparedParameter::U64(3)),
			("source_columns", PreparedParameter::U64(4)),
			("take", PreparedParameter::U64(2)),
			("slice_indices_verified", PreparedParameter::Bool(true)),
		],
		46_000,
	)?;
	assert_fragment(&leading_slice, 2, 2)?;

	let row_slice = materialize_flat_indexing(
		"gpu_slice_rows",
		"slice_indices",
		"sliced",
		12,
		8,
		&[
			("rows", PreparedParameter::U64(3)),
			("columns", PreparedParameter::U64(4)),
			("start", PreparedParameter::U64(1)),
			("count", PreparedParameter::U64(2)),
			("slice_indices_verified", PreparedParameter::Bool(true)),
		],
		48_000,
	)?;
	assert_fragment(&row_slice, 2, 2)?;

	let transpose = materialize_flat_indexing(
		"gpu_transpose",
		"transpose_indices",
		"transposed",
		12,
		12,
		&[
			("rows", PreparedParameter::U64(3)),
			("columns", PreparedParameter::U64(4)),
			("transpose_indices_verified", PreparedParameter::Bool(true)),
		],
		50_000,
	)?;
	assert_fragment(&transpose, 2, 2)?;

	for fragment in [&column_slice, &leading_slice, &row_slice, &transpose] {
		assert_checked_gather(&fragment.graph().nodes[0].kernel.kind)?;
	}
	Ok(())
}

#[test]
fn materializes_encoding_and_triangular_layout_operations() -> TestResult {
	let feature_values = input_tensor(1, DType::F32, &[2, 1])?;
	let bin_indices = input_tensor(2, DType::F32, &[1, 4])?;
	let edges = output_tensor(3, DType::F32, &[2, 4])?;
	let uniform_inputs = [
		NamedTensor::new("feature_values", &feature_values),
		NamedTensor::new("bin_indices", &bin_indices),
	];
	let uniform_outputs = [NamedTensor::new("edges", &edges)];
	let uniform_parameters = parameters(&[
		("rows", PreparedParameter::U64(1)),
		("columns", PreparedParameter::U64(2)),
		("bins", PreparedParameter::U64(3)),
		("bin_indices_verified", PreparedParameter::Bool(true)),
	]);
	let uniform = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_bin_edges_uniform")?,
		&uniform_inputs,
		&uniform_outputs,
		"feature_values",
		&uniform_parameters,
		namespace(60_000),
		ByteCount::new(0),
	))?;
	assert_fragment(&uniform, 1, 1)?;
	assert_eq!(uniform.workspace().bytes(), ByteCount::new(0));
	let uniform_program = elementwise_program(&uniform.graph().nodes[0].kernel.kind)?;
	assert!(
		uniform_program
			.instructions
			.iter()
			.any(|instruction| instruction.opcode == ScalarOpcode::Divide)
	);

	let labels = input_tensor(11, DType::I32, &[3])?;
	let destinations = input_tensor(12, DType::I32, &[3])?;
	let base = input_tensor(13, DType::F32, &[12])?;
	let encoded = output_tensor(14, DType::F32, &[12])?;
	let one_hot_inputs = [
		NamedTensor::new("labels", &labels),
		NamedTensor::new("destination_indices", &destinations),
		NamedTensor::new("output_base", &base),
	];
	let one_hot_outputs = [NamedTensor::new("encoded", &encoded)];
	let one_hot_parameters = parameters(&[
		("rows", PreparedParameter::U64(3)),
		("classes", PreparedParameter::U64(4)),
		(
			"destination_indices_verified_unique",
			PreparedParameter::Bool(true),
		),
	]);
	let one_hot = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_one_hot")?,
		&one_hot_inputs,
		&one_hot_outputs,
		"labels",
		&one_hot_parameters,
		namespace(62_000),
		ByteCount::new(60),
	))?;
	assert_fragment(&one_hot, 2, 3)?;
	assert_eq!(one_hot.workspace().bytes(), ByteCount::new(60));
	let checked_updates = elementwise_program(&one_hot.graph().nodes[1].kernel.kind)?;
	assert!(
		checked_updates
			.instructions
			.iter()
			.any(|instruction| instruction.opcode == ScalarOpcode::Require)
	);
	assert_unique_scatter(&one_hot.graph().nodes[2].kernel.kind)?;

	let factor = input_tensor(21, DType::F32, &[12])?;
	let factor_indices = input_tensor(22, DType::I32, &[9])?;
	let upper_mask = input_tensor(23, DType::I32, &[9])?;
	let zero = input_tensor(24, DType::F32, &[1])?;
	let upper = output_tensor(25, DType::F32, &[9])?;
	let triangle_inputs = [
		NamedTensor::new("factor", &factor),
		NamedTensor::new("factor_indices", &factor_indices),
		NamedTensor::new("upper_mask", &upper_mask),
		NamedTensor::new("zero", &zero),
	];
	let triangle_outputs = [NamedTensor::new("upper_triangle", &upper)];
	let triangle_parameters = parameters(&[
		("factor_rows", PreparedParameter::U64(4)),
		("dimension", PreparedParameter::U64(3)),
		("factor_indices_verified", PreparedParameter::Bool(true)),
		("upper_mask_verified", PreparedParameter::Bool(true)),
		("zero_verified", PreparedParameter::Bool(true)),
	]);
	let triangle = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_pack_upper_tri")?,
		&triangle_inputs,
		&triangle_outputs,
		"factor",
		&triangle_parameters,
		namespace(64_000),
		ByteCount::new(36),
	))?;
	assert_fragment(&triangle, 2, 2)?;
	assert_eq!(triangle.workspace().bytes(), ByteCount::new(36));
	assert_checked_gather(&triangle.graph().nodes[0].kernel.kind)?;
	let triangle_program = elementwise_program(&triangle.graph().nodes[1].kernel.kind)?;
	assert!(
		triangle_program
			.instructions
			.iter()
			.any(|instruction| instruction.opcode == ScalarOpcode::Select)
	);

	let row_indices = input_tensor(31, DType::I32, &[3, 3])?;
	let column_indices = input_tensor(32, DType::I32, &[3, 3])?;
	let fill = input_tensor(33, DType::F32, &[1])?;
	let mask = output_tensor(34, DType::F32, &[3, 3])?;
	let mask_inputs = [
		NamedTensor::new("row_indices", &row_indices),
		NamedTensor::new("column_indices", &column_indices),
		NamedTensor::new("fill_value", &fill),
	];
	let mask_outputs = [NamedTensor::new("mask", &mask)];
	let mask_parameters = parameters(&[
		("dimension", PreparedParameter::U64(3)),
		("coordinate_tables_verified", PreparedParameter::Bool(true)),
	]);
	let triangular_mask = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_tril_mask")?,
		&mask_inputs,
		&mask_outputs,
		"row_indices",
		&mask_parameters,
		namespace(66_000),
		ByteCount::new(0),
	))?;
	assert_fragment(&triangular_mask, 1, 1)?;
	let mask_program = elementwise_program(&triangular_mask.graph().nodes[0].kernel.kind)?;
	assert!(
		mask_program
			.instructions
			.iter()
			.any(|instruction| instruction.opcode == ScalarOpcode::LessThanOrEqual)
	);
	assert!(
		mask_program
			.instructions
			.iter()
			.any(|instruction| instruction.opcode == ScalarOpcode::Select)
	);
	Ok(())
}

#[test]
fn rejects_unverified_tables_and_unsupported_domains() -> TestResult {
	let feature_values = input_tensor(1, DType::F32, &[2, 2])?;
	let bin_indices = input_tensor(2, DType::F32, &[1, 3])?;
	let edges = output_tensor(3, DType::F32, &[2, 3])?;
	let uniform_inputs = [
		NamedTensor::new("feature_values", &feature_values),
		NamedTensor::new("bin_indices", &bin_indices),
	];
	let uniform_outputs = [NamedTensor::new("edges", &edges)];
	let uniform_parameters = parameters(&[
		("rows", PreparedParameter::U64(2)),
		("columns", PreparedParameter::U64(2)),
		("bins", PreparedParameter::U64(2)),
		("bin_indices_verified", PreparedParameter::Bool(true)),
	]);
	let error = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_bin_edges_uniform")?,
		&uniform_inputs,
		&uniform_outputs,
		"feature_values",
		&uniform_parameters,
		namespace(70_000),
		ByteCount::new(0),
	))
	.expect_err("multi-row uniform edges were silently accepted");
	assert_eq!(error.kind, OperationErrorKind::UnsupportedConcreteShape);

	let values = input_tensor(11, DType::F32, &[5])?;
	let prefix = input_tensor(12, DType::I32, &[3])?;
	let indices = output_tensor(13, DType::I32, &[3])?;
	let partial_inputs = [
		NamedTensor::new("values", &values),
		NamedTensor::new("prefix_indices", &prefix),
	];
	let partial_outputs = [NamedTensor::new("indices", &indices)];
	let partial_parameters = parameters(&[
		("elements", PreparedParameter::U64(5)),
		("prefix", PreparedParameter::U64(3)),
		("prefix_indices_verified", PreparedParameter::Bool(true)),
	]);
	let error = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_partial_argsort")?,
		&partial_inputs,
		&partial_outputs,
		"values",
		&partial_parameters,
		namespace(72_000),
		ByteCount::new(40),
	))
	.expect_err("short legacy partial argsort was silently accepted");
	assert_eq!(error.kind, OperationErrorKind::UnsupportedConcreteShape);

	let row_values = input_tensor(21, DType::F32, &[1, 65])?;
	let row_prefix = input_tensor(22, DType::I32, &[65])?;
	let row_indices = output_tensor(23, DType::I32, &[1, 65])?;
	let topk_inputs = [
		NamedTensor::new("values", &row_values),
		NamedTensor::new("prefix_indices", &row_prefix),
	];
	let topk_outputs = [NamedTensor::new("indices", &row_indices)];
	let topk_parameters = parameters(&[
		("rows", PreparedParameter::U64(1)),
		("columns", PreparedParameter::U64(65)),
		("k", PreparedParameter::U64(65)),
		("prefix_indices_verified", PreparedParameter::Bool(true)),
	]);
	let error = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_topk_per_row")?,
		&topk_inputs,
		&topk_outputs,
		"values",
		&topk_parameters,
		namespace(74_000),
		ByteCount::new(520),
	))
	.expect_err("top-k width beyond the legacy local buffer was accepted");
	assert_eq!(error.kind, OperationErrorKind::UnsupportedConcreteShape);

	let slice_values = input_tensor(31, DType::F32, &[12])?;
	let slice_indices = input_tensor(32, DType::I32, &[6])?;
	let sliced = output_tensor(33, DType::F32, &[6])?;
	let slice_inputs = [
		NamedTensor::new("values", &slice_values),
		NamedTensor::new("slice_indices", &slice_indices),
	];
	let slice_outputs = [NamedTensor::new("sliced", &sliced)];
	let unverified_parameters = parameters(&[
		("rows", PreparedParameter::U64(3)),
		("columns", PreparedParameter::U64(4)),
		("start", PreparedParameter::U64(1)),
		("count", PreparedParameter::U64(2)),
		("slice_indices_verified", PreparedParameter::Bool(false)),
	]);
	let error = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_slice_cols")?,
		&slice_inputs,
		&slice_outputs,
		"values",
		&unverified_parameters,
		namespace(76_000),
		ByteCount::new(24),
	))
	.expect_err("unverified slice coordinates were accepted");
	assert_eq!(
		error.kind,
		OperationErrorKind::InvalidMaterializationRequest
	);

	let invalid_range_parameters = parameters(&[
		("rows", PreparedParameter::U64(3)),
		("columns", PreparedParameter::U64(4)),
		("start", PreparedParameter::U64(3)),
		("count", PreparedParameter::U64(2)),
		("slice_indices_verified", PreparedParameter::Bool(true)),
	]);
	let error = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_slice_cols")?,
		&slice_inputs,
		&slice_outputs,
		"values",
		&invalid_range_parameters,
		namespace(78_000),
		ByteCount::new(24),
	))
	.expect_err("out-of-range column slice was accepted");
	assert_eq!(error.kind, OperationErrorKind::UnsupportedConcreteShape);
	Ok(())
}
