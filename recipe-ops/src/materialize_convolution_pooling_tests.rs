use std::collections::BTreeMap;
use std::error::Error;

use recipe_core::{ByteCount, DType, KernelTemplateId, ScalarLiteral, ScalarOpcode, ValueId};
use recipe_language::{AtomicOperation, PrimitiveKind, ReduceOperator, ReduceResult, ScatterConflict, Shape, Tensor};
use recipe_primitives::{ReductionTieBreak, StageKind};

use crate::{
	IdentityNamespace, MaterializationRequest, NamedTensor, OperationErrorKind, PreparedParameter,
	PreparedParameters, channelwise_max_pool_1d_backward_descriptor, channelwise_max_pool_1d_descriptor,
	materialize_composition, operation_registry, prepare_channelwise_max_pool_1d,
};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

fn tensor(id: u64, dtype: DType, extents: &[u64], input: bool, output: bool) -> TestResult<Tensor> {
	Ok(Tensor::contiguous(
		ValueId::new(id),
		dtype,
		Shape::new(extents.to_vec())?,
		input,
		output,
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
		2_000,
		KernelTemplateId::new(seed),
		2_000,
	)
}

fn parameters(values: &[(&str, PreparedParameter)]) -> PreparedParameters {
	values.iter()
		.map(|(name, value)| ((*name).to_owned(), *value))
		.collect()
}

fn assert_graph(materialized: &crate::MaterializedComposition, workspace: u64, nodes: usize) -> TestResult {
	assert_eq!(materialized.workspace().bytes(), ByteCount::new(workspace));
	assert_eq!(materialized.graph().nodes.len(), nodes);
	materialized.graph().validate()?;
	Ok(())
}

fn pool_2d_parameters(extra: &[(&str, PreparedParameter)]) -> PreparedParameters {
	let mut prepared = parameters(&[
		("batch", PreparedParameter::U64(1)),
		("channels", PreparedParameter::U64(1)),
		("input_height", PreparedParameter::U64(3)),
		("input_width", PreparedParameter::U64(3)),
		("kernel_height", PreparedParameter::U64(2)),
		("kernel_width", PreparedParameter::U64(2)),
		("stride_height", PreparedParameter::U64(1)),
		("stride_width", PreparedParameter::U64(1)),
	]);
	prepared.extend(
		extra.iter()
			.map(|(name, value)| ((*name).to_owned(), *value)),
	);
	prepared
}

#[test]
fn materializes_basic_and_extended_image_to_column_gathers() -> TestResult {
	let image = input_tensor(1, DType::F32, &[9])?;
	let indices = input_tensor(2, DType::I32, &[16])?;
	let columns = output_tensor(3, DType::F32, &[16])?;
	let inputs = [
		NamedTensor::new("image", &image),
		NamedTensor::new("receptive_field_indices", &indices),
	];
	let outputs = [NamedTensor::new("columns", &columns)];
	let prepared = parameters(&[
		("batch", PreparedParameter::U64(1)),
		("channels", PreparedParameter::U64(1)),
		("input_height", PreparedParameter::U64(3)),
		("input_width", PreparedParameter::U64(3)),
		("kernel_height", PreparedParameter::U64(2)),
		("kernel_width", PreparedParameter::U64(2)),
		(
			"indices_encode_receptive_fields",
			PreparedParameter::Bool(true),
		),
	]);
	let basic = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_im2col_2d")?,
		&inputs,
		&outputs,
		"image",
		&prepared,
		namespace(10_000),
		ByteCount::new(64),
	))?;
	assert_graph(&basic, 64, 2)?;

	let image = input_tensor(11, DType::F32, &[25])?;
	let indices = input_tensor(12, DType::I32, &[36])?;
	let columns = output_tensor(13, DType::F32, &[36])?;
	let inputs = [
		NamedTensor::new("image", &image),
		NamedTensor::new("receptive_field_indices", &indices),
	];
	let outputs = [NamedTensor::new("columns", &columns)];
	let prepared = parameters(&[
		("batch", PreparedParameter::U64(1)),
		("channels", PreparedParameter::U64(1)),
		("input_height", PreparedParameter::U64(5)),
		("input_width", PreparedParameter::U64(5)),
		("kernel_height", PreparedParameter::U64(3)),
		("kernel_width", PreparedParameter::U64(3)),
		("stride_height", PreparedParameter::U64(2)),
		("stride_width", PreparedParameter::U64(2)),
		("padding_height", PreparedParameter::U64(0)),
		("padding_width", PreparedParameter::U64(0)),
		("dilation_height", PreparedParameter::U64(1)),
		("dilation_width", PreparedParameter::U64(1)),
		(
			"indices_encode_receptive_fields",
			PreparedParameter::Bool(true),
		),
	]);
	let extended = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_im2col_2d_ext")?,
		&inputs,
		&outputs,
		"image",
		&prepared,
		namespace(12_000),
		ByteCount::new(144),
	))?;
	assert_graph(&extended, 144, 2)?;

	let mut padded = prepared;
	padded.insert("padding_height".to_owned(), PreparedParameter::U64(1));
	let error = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_im2col_2d_ext")?,
		&inputs,
		&outputs,
		"image",
		&padded,
		namespace(14_000),
		ByteCount::new(144),
	))
	.expect_err("nonzero-padding im2col unexpectedly materialized");
	assert_eq!(error.kind, OperationErrorKind::UnsupportedConcreteShape);
	Ok(())
}

#[test]
fn materializes_all_column_to_image_scatter_variants() -> TestResult {
	let patches = input_tensor(1, DType::F32, &[6])?;
	let patch_indices = input_tensor(2, DType::I32, &[6])?;
	let destinations = input_tensor(3, DType::I32, &[6])?;
	let base = input_tensor(4, DType::F32, &[4])?;
	let image = output_tensor(5, DType::F32, &[4])?;
	let inputs = [
		NamedTensor::new("patches", &patches),
		NamedTensor::new("patch_indices", &patch_indices),
		NamedTensor::new("destination_indices", &destinations),
		NamedTensor::new("image_base", &base),
	];
	let outputs = [NamedTensor::new("image", &image)];
	let prepared = parameters(&[
		("batch", PreparedParameter::U64(1)),
		("input_length", PreparedParameter::U64(4)),
		("kernel_size", PreparedParameter::U64(2)),
		("patch_indices_verified", PreparedParameter::Bool(true)),
		(
			"destination_indices_verified",
			PreparedParameter::Bool(true),
		),
		("image_base_zero", PreparedParameter::Bool(true)),
	]);
	let one_dimensional = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_col2im_1d")?,
		&inputs,
		&outputs,
		"patches",
		&prepared,
		namespace(20_000),
		ByteCount::new(48),
	))?;
	assert_graph(&one_dimensional, 48, 3)?;

	let patches = input_tensor(11, DType::F32, &[16])?;
	let patch_indices = input_tensor(12, DType::I32, &[16])?;
	let destinations = input_tensor(13, DType::I32, &[16])?;
	let base = input_tensor(14, DType::F32, &[9])?;
	let image = output_tensor(15, DType::F32, &[9])?;
	let inputs = [
		NamedTensor::new("patches", &patches),
		NamedTensor::new("patch_indices", &patch_indices),
		NamedTensor::new("destination_indices", &destinations),
		NamedTensor::new("image_base", &base),
	];
	let outputs = [NamedTensor::new("image", &image)];
	let prepared = parameters(&[
		("batch", PreparedParameter::U64(1)),
		("channels", PreparedParameter::U64(1)),
		("input_height", PreparedParameter::U64(3)),
		("input_width", PreparedParameter::U64(3)),
		("kernel_height", PreparedParameter::U64(2)),
		("kernel_width", PreparedParameter::U64(2)),
		("patch_indices_verified", PreparedParameter::Bool(true)),
		(
			"destination_indices_verified",
			PreparedParameter::Bool(true),
		),
		("image_base_zero", PreparedParameter::Bool(true)),
	]);
	let two_dimensional = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_col2im_2d")?,
		&inputs,
		&outputs,
		"patches",
		&prepared,
		namespace(22_000),
		ByteCount::new(128),
	))?;
	assert_graph(&two_dimensional, 128, 3)?;
	match two_dimensional.graph().nodes[2].kernel.kind {
		PrimitiveKind::Scatter(scatter) => match scatter.conflict {
			ScatterConflict::Atomic { operation, .. } => {
				assert_eq!(operation, AtomicOperation::Add);
			}
			ScatterConflict::UniqueIndices => {
				return Err("overlapping col2im used unique scatter".into());
			}
		},
		_ => return Err("col2im did not end in scatter".into()),
	}

	let patches = input_tensor(21, DType::F32, &[81])?;
	let patch_indices = input_tensor(22, DType::I32, &[49])?;
	let destinations = input_tensor(23, DType::I32, &[49])?;
	let base = input_tensor(24, DType::F32, &[9])?;
	let image = output_tensor(25, DType::F32, &[9])?;
	let inputs = [
		NamedTensor::new("patches", &patches),
		NamedTensor::new("patch_indices", &patch_indices),
		NamedTensor::new("destination_indices", &destinations),
		NamedTensor::new("image_base", &base),
	];
	let outputs = [NamedTensor::new("image", &image)];
	let prepared = parameters(&[
		("batch", PreparedParameter::U64(1)),
		("channels", PreparedParameter::U64(1)),
		("input_height", PreparedParameter::U64(3)),
		("input_width", PreparedParameter::U64(3)),
		("kernel_height", PreparedParameter::U64(3)),
		("kernel_width", PreparedParameter::U64(3)),
		("stride_height", PreparedParameter::U64(1)),
		("stride_width", PreparedParameter::U64(1)),
		("padding_height", PreparedParameter::U64(1)),
		("padding_width", PreparedParameter::U64(1)),
		("dilation_height", PreparedParameter::U64(1)),
		("dilation_width", PreparedParameter::U64(1)),
		("valid_contributions", PreparedParameter::U64(49)),
		("patch_indices_verified", PreparedParameter::Bool(true)),
		(
			"destination_indices_verified",
			PreparedParameter::Bool(true),
		),
		("image_base_zero", PreparedParameter::Bool(true)),
	]);
	let extended = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_col2im_2d_ext")?,
		&inputs,
		&outputs,
		"patches",
		&prepared,
		namespace(24_000),
		ByteCount::new(392),
	))?;
	assert_graph(&extended, 392, 3)
}

#[test]
fn materializes_every_average_pool_forward_alias() -> TestResult {
	let values = input_tensor(1, DType::F32, &[12])?;
	let indices = input_tensor(2, DType::I32, &[4, 3])?;
	let pooled = output_tensor(3, DType::F32, &[4])?;
	let inputs = [
		NamedTensor::new("values", &values),
		NamedTensor::new("window_indices", &indices),
	];
	let outputs = [NamedTensor::new("pooled", &pooled)];
	let prepared = parameters(&[
		("batch", PreparedParameter::U64(2)),
		("window_length", PreparedParameter::U64(3)),
		("filters", PreparedParameter::U64(2)),
		("tree_lanes", PreparedParameter::U64(32)),
		("window_indices_verified", PreparedParameter::Bool(true)),
	]);
	let one_dimensional = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_avg_pool_1d")?,
		&inputs,
		&outputs,
		"values",
		&prepared,
		namespace(30_000),
		ByteCount::new(64),
	))?;
	assert_graph(&one_dimensional, 64, 3)?;

	for (ordinal, symbol) in ["gpu_avg_pool_2d", "gpu_avg_pool_2d_f32"]
		.into_iter()
		.enumerate()
	{
		let values = input_tensor(11, DType::F32, &[9])?;
		let indices = input_tensor(12, DType::I32, &[4, 4])?;
		let pooled = output_tensor(13, DType::F32, &[4])?;
		let inputs = [
			NamedTensor::new("values", &values),
			NamedTensor::new("window_indices", &indices),
		];
		let outputs = [NamedTensor::new("pooled", &pooled)];
		let prepared = pool_2d_parameters(&[
			("tree_lanes", PreparedParameter::U64(32)),
			("window_indices_verified", PreparedParameter::Bool(true)),
		]);
		let materialized = materialize_composition(MaterializationRequest::new(
			operation_registry().resolve_unique(symbol)?,
			&inputs,
			&outputs,
			"values",
			&prepared,
			namespace(32_000 + u64::try_from(ordinal)? * 2_000),
			ByteCount::new(80),
		))?;
		assert_graph(&materialized, 80, 3)?;
	}
	Ok(())
}

#[test]
fn materializes_every_max_pool_forward_alias_with_int32_winners() -> TestResult {
	let values = input_tensor(1, DType::F32, &[12])?;
	let indices = input_tensor(2, DType::I32, &[4, 3])?;
	let pooled = output_tensor(3, DType::F32, &[4])?;
	let winners = output_tensor(4, DType::I32, &[4])?;
	let inputs = [
		NamedTensor::new("values", &values),
		NamedTensor::new("window_indices", &indices),
	];
	let outputs = [
		NamedTensor::new("pooled", &pooled),
		NamedTensor::new("winning_indices", &winners),
	];
	let prepared = parameters(&[
		("batch", PreparedParameter::U64(2)),
		("window_length", PreparedParameter::U64(3)),
		("filters", PreparedParameter::U64(2)),
		("tree_lanes", PreparedParameter::U64(32)),
		("window_indices_verified", PreparedParameter::Bool(true)),
	]);
	let one_dimensional = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_max_pool_1d")?,
		&inputs,
		&outputs,
		"values",
		&prepared,
		namespace(40_000),
		ByteCount::new(80),
	))?;
	assert_graph(&one_dimensional, 80, 3)?;

	for (ordinal, symbol) in ["gpu_max_pool_2d", "gpu_max_pool_2d_f32"]
		.into_iter()
		.enumerate()
	{
		let values = input_tensor(11, DType::F32, &[9])?;
		let indices = input_tensor(12, DType::I32, &[4, 4])?;
		let bases = input_tensor(13, DType::I32, &[4])?;
		let pooled = output_tensor(14, DType::F32, &[4])?;
		let winners = output_tensor(15, DType::I32, &[4])?;
		let inputs = [
			NamedTensor::new("values", &values),
			NamedTensor::new("window_indices", &indices),
			NamedTensor::new("window_bases", &bases),
		];
		let outputs = [
			NamedTensor::new("pooled", &pooled),
			NamedTensor::new("winning_indices", &winners),
		];
		let prepared = pool_2d_parameters(&[
			("tree_lanes", PreparedParameter::U64(32)),
			("window_indices_verified", PreparedParameter::Bool(true)),
			("window_bases_verified", PreparedParameter::Bool(true)),
		]);
		let materialized = materialize_composition(MaterializationRequest::new(
			operation_registry().resolve_unique(symbol)?,
			&inputs,
			&outputs,
			"values",
			&prepared,
			namespace(42_000 + u64::try_from(ordinal)? * 2_000),
			ByteCount::new(96),
		))?;
		assert_graph(&materialized, 96, 3)?;
	}
	Ok(())
}

#[test]
fn materializes_average_pool_backward_aliases_and_gradient_expand() -> TestResult {
	for (ordinal, symbol) in ["gpu_avg_pool_2d_backward", "gpu_avg_pool_2d_backward_f32"]
		.into_iter()
		.enumerate()
	{
		let output_gradient = input_tensor(1, DType::F32, &[4])?;
		let gradient_indices = input_tensor(2, DType::I32, &[16])?;
		let destinations = input_tensor(3, DType::I32, &[16])?;
		let base = input_tensor(4, DType::F32, &[9])?;
		let input_gradient = output_tensor(5, DType::F32, &[9])?;
		let inputs = [
			NamedTensor::new("output_gradient", &output_gradient),
			NamedTensor::new("gradient_indices", &gradient_indices),
			NamedTensor::new("destination_indices", &destinations),
			NamedTensor::new("input_gradient_base", &base),
		];
		let outputs = [NamedTensor::new("input_gradient", &input_gradient)];
		let prepared = pool_2d_parameters(&[
			("gradient_indices_verified", PreparedParameter::Bool(true)),
			(
				"destination_indices_verified",
				PreparedParameter::Bool(true),
			),
			("input_gradient_base_zero", PreparedParameter::Bool(true)),
		]);
		let materialized = materialize_composition(MaterializationRequest::new(
			operation_registry().resolve_unique(symbol)?,
			&inputs,
			&outputs,
			"output_gradient",
			&prepared,
			namespace(50_000 + u64::try_from(ordinal)? * 2_000),
			ByteCount::new(128),
		))?;
		assert_graph(&materialized, 128, 3)?;
	}

	let output_gradient = input_tensor(11, DType::F32, &[4])?;
	let gradient_indices = input_tensor(12, DType::I32, &[12])?;
	let destinations = input_tensor(13, DType::I32, &[12])?;
	let base = input_tensor(14, DType::F32, &[12])?;
	let input_gradient = output_tensor(15, DType::F32, &[12])?;
	let inputs = [
		NamedTensor::new("output_gradient", &output_gradient),
		NamedTensor::new("gradient_indices", &gradient_indices),
		NamedTensor::new("destination_indices", &destinations),
		NamedTensor::new("input_gradient_base", &base),
	];
	let outputs = [NamedTensor::new("input_gradient", &input_gradient)];
	let prepared = parameters(&[
		("batch", PreparedParameter::U64(2)),
		("window_length", PreparedParameter::U64(3)),
		("filters", PreparedParameter::U64(2)),
		("gradient_indices_verified", PreparedParameter::Bool(true)),
		(
			"destination_indices_verified",
			PreparedParameter::Bool(true),
		),
		("destination_indices_unique", PreparedParameter::Bool(true)),
		("input_gradient_base_zero", PreparedParameter::Bool(true)),
	]);
	let expanded = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_pool_grad_expand")?,
		&inputs,
		&outputs,
		"output_gradient",
		&prepared,
		namespace(54_000),
		ByteCount::new(96),
	))?;
	assert_graph(&expanded, 96, 3)
}

#[test]
fn materializes_every_max_pool_backward_alias_with_explicit_race_policy() -> TestResult {
	let output_gradient = input_tensor(1, DType::F32, &[4])?;
	let winners = input_tensor(2, DType::I32, &[4])?;
	let gradient_indices = input_tensor(3, DType::I32, &[4])?;
	let bases = input_tensor(4, DType::I32, &[4])?;
	let base = input_tensor(5, DType::F32, &[12])?;
	let input_gradient = output_tensor(6, DType::F32, &[12])?;
	let inputs = [
		NamedTensor::new("output_gradient", &output_gradient),
		NamedTensor::new("winning_indices", &winners),
		NamedTensor::new("gradient_indices", &gradient_indices),
		NamedTensor::new("destination_bases", &bases),
		NamedTensor::new("input_gradient_base", &base),
	];
	let outputs = [NamedTensor::new("input_gradient", &input_gradient)];
	let prepared = parameters(&[
		("batch", PreparedParameter::U64(2)),
		("window_length", PreparedParameter::U64(3)),
		("filters", PreparedParameter::U64(2)),
		("gradient_indices_verified", PreparedParameter::Bool(true)),
		("destination_bases_verified", PreparedParameter::Bool(true)),
		("destination_indices_unique", PreparedParameter::Bool(true)),
		("input_gradient_base_zero", PreparedParameter::Bool(true)),
	]);
	let one_dimensional = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_max_pool_1d_backward")?,
		&inputs,
		&outputs,
		"output_gradient",
		&prepared,
		namespace(60_000),
		ByteCount::new(48),
	))?;
	assert_graph(&one_dimensional, 48, 3)?;
	match one_dimensional.graph().nodes[2].kernel.kind {
		PrimitiveKind::Scatter(scatter) => {
			assert_eq!(scatter.conflict, ScatterConflict::UniqueIndices);
		}
		_ => return Err("one-dimensional max backward did not scatter".into()),
	}

	for (ordinal, symbol) in ["gpu_max_pool_2d_backward", "gpu_max_pool_2d_backward_f32"]
		.into_iter()
		.enumerate()
	{
		let output_gradient = input_tensor(11, DType::F32, &[4])?;
		let winners = input_tensor(12, DType::I32, &[4])?;
		let gradient_indices = input_tensor(13, DType::I32, &[4])?;
		let bases = input_tensor(14, DType::I32, &[4])?;
		let base = input_tensor(15, DType::F32, &[9])?;
		let input_gradient = output_tensor(16, DType::F32, &[9])?;
		let inputs = [
			NamedTensor::new("output_gradient", &output_gradient),
			NamedTensor::new("winning_indices", &winners),
			NamedTensor::new("gradient_indices", &gradient_indices),
			NamedTensor::new("plane_bases", &bases),
			NamedTensor::new("input_gradient_base", &base),
		];
		let outputs = [NamedTensor::new("input_gradient", &input_gradient)];
		let prepared = parameters(&[
			("batch", PreparedParameter::U64(1)),
			("channels", PreparedParameter::U64(1)),
			("input_height", PreparedParameter::U64(3)),
			("input_width", PreparedParameter::U64(3)),
			("output_height", PreparedParameter::U64(2)),
			("output_width", PreparedParameter::U64(2)),
			("gradient_indices_verified", PreparedParameter::Bool(true)),
			("plane_bases_verified", PreparedParameter::Bool(true)),
			("input_gradient_base_zero", PreparedParameter::Bool(true)),
		]);
		let materialized = materialize_composition(MaterializationRequest::new(
			operation_registry().resolve_unique(symbol)?,
			&inputs,
			&outputs,
			"output_gradient",
			&prepared,
			namespace(62_000 + u64::try_from(ordinal)? * 2_000),
			ByteCount::new(48),
		))?;
		assert_graph(&materialized, 48, 3)?;
		match materialized.graph().nodes[2].kernel.kind {
			PrimitiveKind::Scatter(scatter) => match scatter.conflict {
				ScatterConflict::Atomic { operation, .. } => {
					assert_eq!(operation, AtomicOperation::Add);
				}
				ScatterConflict::UniqueIndices => {
					return Err("overlapping max-pool backward used unique scatter".into());
				}
			},
			_ => return Err("two-dimensional max backward did not scatter".into()),
		}
	}
	Ok(())
}

#[test]
fn materializes_channelwise_nonoverlapping_max_pool_with_short_tail_and_global_winners() -> TestResult {
	let preparation = prepare_channelwise_max_pool_1d(2, 5, 2, 3)?;
	let values = input_tensor(101, DType::F32, &preparation.input_shape())?;
	let indices = input_tensor(102, DType::I32, &preparation.window_indices_shape())?;
	let bases = input_tensor(103, DType::I32, &preparation.output_shape())?;
	let pooled = output_tensor(104, DType::F32, &preparation.output_shape())?;
	let winners = output_tensor(105, DType::I32, &preparation.output_shape())?;
	let inputs = [
		NamedTensor::new("values", &values),
		NamedTensor::new("window_indices", &indices),
		NamedTensor::new("winner_bases", &bases),
	];
	let outputs = [
		NamedTensor::new("pooled", &pooled),
		NamedTensor::new("winning_indices", &winners),
	];
	let materialized = materialize_composition(MaterializationRequest::new(
		channelwise_max_pool_1d_descriptor(),
		&inputs,
		&outputs,
		"values",
		&preparation.forward_parameters(4),
		namespace(80_000),
		preparation.forward_workspace(),
	))?;
	assert_graph(&materialized, 160, 3)?;

	let reduction = materialized
		.graph()
		.nodes
		.iter()
		.find(|node| matches!(node.kernel.kind, PrimitiveKind::Reduce(_)))
		.ok_or("channelwise pool omitted its maximum reduction")?;
	let PrimitiveKind::Reduce(spec) = &reduction.kernel.kind else {
		unreachable!();
	};
	assert_eq!(spec.operator, ReduceOperator::Maximum);
	assert_eq!(spec.result, ReduceResult::ValueAndIndex);
	assert_eq!(spec.axes.as_slice(), [3]);
	let PrimitiveKind::Elementwise(result_map) = &materialized.graph().nodes[2].kernel.kind else {
		return Err("channelwise pool omitted its global-winner map".into());
	};
	assert_eq!(
		result_map
			.program
			.constants
			.iter()
			.map(|constant| constant.value)
			.collect::<Vec<_>>(),
		[ScalarLiteral::I32(2)]
	);
	assert_eq!(
		result_map
			.program
			.instructions
			.iter()
			.map(|instruction| instruction.opcode)
			.collect::<Vec<_>>(),
		[ScalarOpcode::Multiply, ScalarOpcode::Add]
	);
	let tensors = materialized
		.graph()
		.tensors
		.iter()
		.map(|tensor| (tensor.id, tensor))
		.collect::<BTreeMap<_, _>>();
	let lowered = recipe_primitives::lower(&reduction.kernel, &tensors)?;
	assert!(lowered.stages.iter().all(|stage| match &stage.kind {
		StageKind::FixedTreeReduce(stage) => stage.tie_break == ReductionTieBreak::LowestLogicalIndex,
		_ => false,
	}));
	Ok(())
}

#[test]
fn materializes_channelwise_max_pool_backward_as_one_unique_global_scatter() -> TestResult {
	let preparation = prepare_channelwise_max_pool_1d(2, 5, 2, 3)?;
	let output_gradient = input_tensor(111, DType::F32, &preparation.output_shape())?;
	let winners = input_tensor(112, DType::I32, &preparation.output_shape())?;
	let batch_indices = input_tensor(113, DType::I32, &[preparation.batch()])?;
	let base = input_tensor(114, DType::F32, &preparation.input_shape())?;
	let input_gradient = output_tensor(115, DType::F32, &preparation.input_shape())?;
	let inputs = [
		NamedTensor::new("output_gradient", &output_gradient),
		NamedTensor::new("winning_indices", &winners),
		NamedTensor::new("gradient_batch_indices", &batch_indices),
		NamedTensor::new("input_gradient_base", &base),
	];
	let outputs = [NamedTensor::new("input_gradient", &input_gradient)];
	let materialized = materialize_composition(MaterializationRequest::new(
		channelwise_max_pool_1d_backward_descriptor(),
		&inputs,
		&outputs,
		"output_gradient",
		&preparation.backward_parameters(),
		namespace(82_000),
		preparation.backward_workspace(),
	))?;
	assert_graph(&materialized, 96, 3)?;
	let PrimitiveKind::Scatter(scatter) = materialized.graph().nodes[2].kernel.kind else {
		return Err("channelwise maximum-pool backward did not scatter".into());
	};
	assert_eq!(scatter.conflict, ScatterConflict::UniqueIndices);
	Ok(())
}

#[test]
fn materializes_size_one_as_shape_preserving_forward_and_backward() -> TestResult {
	let preparation = prepare_channelwise_max_pool_1d(2, 3, 2, 1)?;
	let values = input_tensor(131, DType::F32, &preparation.input_shape())?;
	let indices = input_tensor(132, DType::I32, &preparation.window_indices_shape())?;
	let bases = input_tensor(133, DType::I32, &preparation.output_shape())?;
	let pooled = output_tensor(134, DType::F32, &preparation.output_shape())?;
	let winners = output_tensor(135, DType::I32, &preparation.output_shape())?;
	let forward_inputs = [
		NamedTensor::new("values", &values),
		NamedTensor::new("window_indices", &indices),
		NamedTensor::new("winner_bases", &bases),
	];
	let forward_outputs = [
		NamedTensor::new("pooled", &pooled),
		NamedTensor::new("winning_indices", &winners),
	];
	let forward = materialize_composition(MaterializationRequest::new(
		channelwise_max_pool_1d_descriptor(),
		&forward_inputs,
		&forward_outputs,
		"values",
		&preparation.forward_parameters(1),
		namespace(86_000),
		preparation.forward_workspace(),
	))?;
	assert_eq!(preparation.output_shape(), [2, 3, 2]);
	assert_graph(&forward, 144, 3)?;

	let output_gradient = input_tensor(141, DType::F32, &preparation.output_shape())?;
	let winners = input_tensor(142, DType::I32, &preparation.output_shape())?;
	let batch_indices = input_tensor(143, DType::I32, &[preparation.batch()])?;
	let gradient_base = input_tensor(144, DType::F32, &preparation.input_shape())?;
	let input_gradient = output_tensor(145, DType::F32, &preparation.input_shape())?;
	let backward_inputs = [
		NamedTensor::new("output_gradient", &output_gradient),
		NamedTensor::new("winning_indices", &winners),
		NamedTensor::new("gradient_batch_indices", &batch_indices),
		NamedTensor::new("input_gradient_base", &gradient_base),
	];
	let backward_outputs = [NamedTensor::new("input_gradient", &input_gradient)];
	let backward = materialize_composition(MaterializationRequest::new(
		channelwise_max_pool_1d_backward_descriptor(),
		&backward_inputs,
		&backward_outputs,
		"output_gradient",
		&preparation.backward_parameters(),
		namespace(88_000),
		preparation.backward_workspace(),
	))?;
	assert_graph(&backward, 144, 3)
}

#[test]
fn channelwise_pool_requires_the_exact_tail_proof() -> TestResult {
	let preparation = prepare_channelwise_max_pool_1d(1, 5, 2, 2)?;
	let values = input_tensor(121, DType::F32, &preparation.input_shape())?;
	let indices = input_tensor(122, DType::I32, &preparation.window_indices_shape())?;
	let bases = input_tensor(123, DType::I32, &preparation.output_shape())?;
	let pooled = output_tensor(124, DType::F32, &preparation.output_shape())?;
	let winners = output_tensor(125, DType::I32, &preparation.output_shape())?;
	let inputs = [
		NamedTensor::new("values", &values),
		NamedTensor::new("window_indices", &indices),
		NamedTensor::new("winner_bases", &bases),
	];
	let outputs = [
		NamedTensor::new("pooled", &pooled),
		NamedTensor::new("winning_indices", &winners),
	];
	let mut parameters = preparation.forward_parameters(4);
	parameters.insert(
		"tail_window_repeats_last_coordinate".to_owned(),
		PreparedParameter::Bool(false),
	);
	let error = materialize_composition(MaterializationRequest::new(
		channelwise_max_pool_1d_descriptor(),
		&inputs,
		&outputs,
		"values",
		&parameters,
		namespace(84_000),
		preparation.forward_workspace(),
	))
	.expect_err("channelwise pool accepted an unproved short-tail table");
	assert_eq!(
		error.kind,
		OperationErrorKind::InvalidMaterializationRequest
	);
	Ok(())
}

#[test]
fn materializes_checked_nearest_neighbor_upsampling() -> TestResult {
	let values = input_tensor(1, DType::F32, &[4])?;
	let indices = input_tensor(2, DType::I32, &[16])?;
	let upsampled = output_tensor(3, DType::F32, &[16])?;
	let inputs = [
		NamedTensor::new("values", &values),
		NamedTensor::new("source_indices", &indices),
	];
	let outputs = [NamedTensor::new("upsampled", &upsampled)];
	let prepared = parameters(&[
		("batch", PreparedParameter::U64(1)),
		("channels", PreparedParameter::U64(1)),
		("input_height", PreparedParameter::U64(2)),
		("input_width", PreparedParameter::U64(2)),
		("scale_height", PreparedParameter::U64(2)),
		("scale_width", PreparedParameter::U64(2)),
		("source_indices_verified", PreparedParameter::Bool(true)),
	]);
	let materialized = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_upsample_nearest_2d")?,
		&inputs,
		&outputs,
		"values",
		&prepared,
		namespace(70_000),
		ByteCount::new(64),
	))?;
	assert_graph(&materialized, 64, 2)
}
