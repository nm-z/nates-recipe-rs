use recipe_core::{DType, ScalarOpcode}; use recipe_language::{
	AtomicOperation, AtomicOrdering, AxisSet, Elementwise, Gather, IndexBounds, PrimitiveKind, Reduce,
	ReduceOperator, ReduceResult, Scatter, ScatterConflict, };

use super::{
	Emitter, FamilyDispatch, MAX_I32_INDEX, MaterializationRequest, emit_checked_gather_identity, exact_f32_extent,
	identity_program, input, language_error, operation_error, output, prepared_tree_lanes, prepared_u64,
	request_error, require_dtype, require_exact_abi, require_same_tensor_contract, require_shape, require_true,
	scalar_binary, scalar_builder, scalar_f32, scalar_finish, scalar_input, };
use crate::{OperationDescriptor, OperationErrorKind, OperationResult};

const OPERATIONS: &[(&str, &str)] = &[
	("gpu_avg_pool_1d", "gpu-core/src/kernels.rs:4596"),
	("gpu_avg_pool_2d", "gpu-core/src/kernels.rs:5962"),
	("gpu_avg_pool_2d_backward", "gpu-core/src/kernels.rs:5999"),
	("gpu_avg_pool_2d_backward_f32", "gpu-core/src/nn_f32.rs:427"),
	("gpu_avg_pool_2d_f32", "gpu-core/src/nn_f32.rs:391"),
	("gpu_col2im_1d", "gpu-core/src/kernels.rs:5847"),
	("gpu_col2im_2d", "gpu-core/src/kernels.rs:5877"),
	("gpu_col2im_2d_ext", "gpu-core/src/attention.rs:384"),
	("gpu_im2col_1d", "gpu-core/src/kernels.rs:5097"),
	("gpu_im2col_2d", "gpu-core/src/kernels.rs:5417"),
	("gpu_im2col_2d_ext", "gpu-core/src/attention.rs:341"),
	("gpu_max_pool_1d", "gpu-core/src/kernels.rs:5911"),
	("gpu_max_pool_1d_backward", "gpu-core/src/kernels.rs:5936"),
	("gpu_max_pool_2d", "gpu-core/src/kernels.rs:6037"),
	("gpu_max_pool_2d_backward", "gpu-core/src/kernels.rs:6076"),
	("gpu_max_pool_2d_backward_f32", "gpu-core/src/nn_f32.rs:501"),
	("gpu_max_pool_2d_f32", "gpu-core/src/nn_f32.rs:463"),
	("gpu_pool_grad_expand", "gpu-core/src/kernels.rs:4622"),
	("gpu_upsample_nearest_2d", "gpu-core/src/kernels.rs:6620"),
	(
		"recipe_max_pool_1d",
		"ops/src/pooling.rs:channelwise_max_pool_1d",
	), (
		"recipe_max_pool_1d_backward",
		"ops/src/pooling.rs:channelwise_max_pool_1d_backward",
	), ];

#[derive(Clone, Copy, Debug)]
struct PoolGeometry { input_elements: u64, output_elements: u64, window_elements: u64, input_width: u64,
	kernel_width: u64, }

#[derive(Clone, Copy, Debug)]
struct ChannelwisePool1dGeometry { batch: u64, channels: u64, groups: u64, window_width: u64, input_elements: u64, }

pub(super) fn supports(descriptor: OperationDescriptor) -> bool { OPERATIONS.contains(&(descriptor.symbol, descriptor.source)) }

pub(super) fn dispatch(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> FamilyDispatch {
	if !supports(request.descriptor) { return FamilyDispatch::NotOwned; }
	let result = match request.descriptor.symbol {
		"gpu_avg_pool_1d" => emit_average_pool_1d(request, emitter),
		"gpu_avg_pool_2d" | "gpu_avg_pool_2d_f32" => emit_average_pool_2d(request, emitter),
		"gpu_avg_pool_2d_backward" | "gpu_avg_pool_2d_backward_f32" => {
			emit_average_pool_2d_backward(request, emitter) }
		"gpu_col2im_1d" => emit_col2im_1d(request, emitter),
		"gpu_col2im_2d" => emit_col2im_2d(request, emitter, false),
		"gpu_col2im_2d_ext" => emit_col2im_2d(request, emitter, true),
		"gpu_im2col_1d" => {
			emit_checked_gather_identity( request, emitter,
				"image",
				"receptive_field_indices",
				"columns",
				"axis",
				Some("indices_encode_receptive_fields"),
			) }
		"gpu_im2col_2d" => emit_im2col_2d(request, emitter, false),
		"gpu_im2col_2d_ext" => emit_im2col_2d(request, emitter, true),
		"gpu_max_pool_1d" => emit_legacy_max_pool_1d(request, emitter),
		"gpu_max_pool_1d_backward" => emit_legacy_max_pool_1d_backward(request, emitter),
		"gpu_max_pool_2d" | "gpu_max_pool_2d_f32" => emit_max_pool_2d(request, emitter),
		"gpu_max_pool_2d_backward" | "gpu_max_pool_2d_backward_f32" => emit_max_pool_2d_backward(request, emitter),
		"gpu_pool_grad_expand" => emit_pool_gradient_expand(request, emitter),
		"gpu_upsample_nearest_2d" => emit_upsample_nearest_2d(request, emitter),
		"recipe_max_pool_1d" => emit_channelwise_max_pool_1d(request, emitter),
		"recipe_max_pool_1d_backward" => emit_channelwise_max_pool_1d_backward(request, emitter),
		symbol => { Err(operation_error( request.descriptor.id, OperationErrorKind::GraphMaterializationFailed,
				format!("convolution/pooling dispatch is incomplete for {symbol}"),
			)) } }; FamilyDispatch::Owned(result) }

fn emit_im2col_2d(
	request: &MaterializationRequest<'_>,
	emitter: &mut Emitter<'_>,
	extended: bool, ) -> OperationResult<()> { let geometry = match extended { true => { require_exact_abi( request,
				&["image", "receptive_field_indices"],
				&["columns"],
				&[
					"batch",
					"channels",
					"input_height",
					"input_width",
					"kernel_height",
					"kernel_width",
					"stride_height",
					"stride_width",
					"padding_height",
					"padding_width",
					"dilation_height",
					"dilation_width",
					"indices_encode_receptive_fields",
				], )?;
			let padding_height = prepared_u64(request.descriptor.id, request.parameters, "padding_height")?;
			let padding_width = prepared_u64(request.descriptor.id, request.parameters, "padding_width")?;
			if padding_height != 0 || padding_width != 0 { return Err(operation_error( request.descriptor.id,
					OperationErrorKind::UnsupportedConcreteShape,
					"gpu_im2col_2d_ext currently requires explicit zero padding; nonzero padding needs a zero-fill primitive",
				)); }
			convolution_2d_geometry(request, true)? }
		false => { require_exact_abi( request,
				&["image", "receptive_field_indices"],
				&["columns"],
				&[
					"batch",
					"channels",
					"input_height",
					"input_width",
					"kernel_height",
					"kernel_width",
					"indices_encode_receptive_fields",
				], )?; convolution_2d_geometry(request, false)? } };
	require_true(request, "indices_encode_receptive_fields")?;
	let image = input(request, "image")?;
	let indices = input(request, "receptive_field_indices")?;
	let columns = output(request, "columns")?;
	require_dtype(request, image, DType::F32, "image")?;
	require_dtype(request, indices, DType::I32, "receptive_field_indices")?;
	require_dtype(request, columns, DType::F32, "columns")?;
	require_shape(request, image, &[geometry.input_elements], "image")?;
	let patch_elements = checked_product( request, &[geometry.output_elements, geometry.window_elements],
		"lowered image element count",
	)?; require_shape( request, indices, &[patch_elements],
		"receptive_field_indices",
	)?;
	require_shape(request, columns, &[patch_elements], "columns")?;
	emit_checked_flat_gather(request, emitter, image, indices, columns) }

fn emit_col2im_1d(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi( request, &[
			"patches",
			"patch_indices",
			"destination_indices",
			"image_base",
		],
		&["image"],
		&[
			"batch",
			"input_length",
			"kernel_size",
			"patch_indices_verified",
			"destination_indices_verified",
			"image_base_zero",
		], )?;
	let batch = prepared_extent(request, "batch")?;
	let input_length = prepared_extent(request, "input_length")?;
	let kernel_size = prepared_extent(request, "kernel_size")?;
	if kernel_size > input_length { return Err(operation_error( request.descriptor.id,
			OperationErrorKind::UnsupportedConcreteShape,
			"kernel_size must not exceed input_length",
		)); }
	let output_length = input_length - kernel_size + 1; let image_elements = checked_product( request,
		&[batch, input_length],
		"col2im image element count",
	)?; let contribution_count = checked_product( request, &[batch, output_length, kernel_size],
		"col2im contribution count",
	)?; emit_column_scatter( request, emitter, contribution_count, image_elements,
		"patch_indices_verified",
	) }

fn emit_col2im_2d(
	request: &MaterializationRequest<'_>,
	emitter: &mut Emitter<'_>,
	extended: bool, ) -> OperationResult<()> { let (geometry, valid_contributions) = match extended { true => {
			require_exact_abi( request, &[
					"patches",
					"patch_indices",
					"destination_indices",
					"image_base",
				],
				&["image"],
				&[
					"batch",
					"channels",
					"input_height",
					"input_width",
					"kernel_height",
					"kernel_width",
					"stride_height",
					"stride_width",
					"padding_height",
					"padding_width",
					"dilation_height",
					"dilation_width",
					"valid_contributions",
					"patch_indices_verified",
					"destination_indices_verified",
					"image_base_zero",
				], )?; let geometry = convolution_2d_geometry(request, true)?;
			let valid = prepared_extent(request, "valid_contributions")?;
			let patch_elements = checked_product( request, &[geometry.output_elements, geometry.window_elements],
				"extended col2im patch element count",
			)?; if valid > patch_elements { return Err(request_error( request,
					"valid_contributions must not exceed the complete patch tensor",
				)); }
			(geometry, valid) }
		false => { require_exact_abi( request, &[
					"patches",
					"patch_indices",
					"destination_indices",
					"image_base",
				],
				&["image"],
				&[
					"batch",
					"channels",
					"input_height",
					"input_width",
					"kernel_height",
					"kernel_width",
					"patch_indices_verified",
					"destination_indices_verified",
					"image_base_zero",
				], )?; let geometry = convolution_2d_geometry(request, false)?; let valid = checked_product( request,
				&[geometry.output_elements, geometry.window_elements],
				"col2im contribution count",
			)?; (geometry, valid) } }; let patch_elements = checked_product( request,
		&[geometry.output_elements, geometry.window_elements],
		"col2im patch element count",
	)?; emit_column_scatter_with_patch_shape( request, emitter, patch_elements, valid_contributions,
		geometry.input_elements,
		"patch_indices_verified",
	) }

fn emit_column_scatter(
	request: &MaterializationRequest<'_>,
	emitter: &mut Emitter<'_>,
	contribution_count: u64, image_elements: u64, index_fact: &str, ) -> OperationResult<()> {
	emit_column_scatter_with_patch_shape( request, emitter, contribution_count, contribution_count, image_elements,
		index_fact, ) }

fn emit_column_scatter_with_patch_shape(
	request: &MaterializationRequest<'_>,
	emitter: &mut Emitter<'_>,
	patch_elements: u64, contribution_count: u64, image_elements: u64, index_fact: &str, ) -> OperationResult<()> {
	require_true(request, index_fact)?;
	require_true(request, "destination_indices_verified")?;
	require_true(request, "image_base_zero")?;
	let patches = input(request, "patches")?;
	let patch_indices = input(request, "patch_indices")?;
	let destinations = input(request, "destination_indices")?;
	let image_base = input(request, "image_base")?;
	let image = output(request, "image")?;
	require_dtype(request, patches, DType::F32, "patches")?;
	require_dtype(request, patch_indices, DType::I32, "patch_indices")?;
	require_dtype(request, destinations, DType::I32, "destination_indices")?;
	require_dtype(request, image_base, DType::F32, "image_base")?;
	require_dtype(request, image, DType::F32, "image")?;
	require_shape(request, patches, &[patch_elements], "patches")?;
	require_shape( request, patch_indices, &[contribution_count],
		"patch_indices",
	)?; require_shape( request, destinations, &[contribution_count],
		"destination_indices",
	)?;
	require_shape(request, image_base, &[image_elements], "image_base")?;
	require_same_tensor_contract(request, image_base, image, "image")?;

	let gathered = emitter.intermediate(DType::F32, patch_indices.shape.clone())?; emitter.emit(
		vec![patches.id, patch_indices.id], vec![gathered], PrimitiveKind::Gather(Gather { axis: 0,
			bounds: IndexBounds::Reject, }), )?; let mapped = emitter.intermediate(DType::F32, patch_indices.shape.clone())?;
	emitter.emit( vec![gathered], vec![mapped], PrimitiveKind::Elementwise(Elementwise {
			program: identity_program(request.descriptor.id, DType::F32)?, }), )?; emitter.emit(
		vec![image_base.id, destinations.id, mapped], vec![image.id], PrimitiveKind::Scatter(Scatter { axis: 0,
			bounds: IndexBounds::Reject, conflict: ScatterConflict::Atomic { operation: AtomicOperation::Add,
				ordering: AtomicOrdering::Relaxed, }, }), ) }

fn emit_average_pool_1d(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(request, &["values", "window_indices"], &["pooled"], &[
		"batch",
		"window_length",
		"filters",
		"tree_lanes",
		"window_indices_verified",
	])?;
	let batch = prepared_extent(request, "batch")?;
	let window = prepared_extent(request, "window_length")?;
	let filters = prepared_extent(request, "filters")?;
	let geometry = PoolGeometry { input_elements: checked_product( request, &[batch, window, filters],
			"average-pool input element count",
		)?, output_elements: checked_product( request, &[batch, filters],
			"average-pool output element count",
		)?, window_elements: window, input_width: window, kernel_width: window, };
	emit_average_pool(request, emitter, geometry) }

fn emit_average_pool_2d(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(request, &["values", "window_indices"], &["pooled"], &[
		"batch",
		"channels",
		"input_height",
		"input_width",
		"kernel_height",
		"kernel_width",
		"stride_height",
		"stride_width",
		"tree_lanes",
		"window_indices_verified",
	])?; let geometry = pooling_2d_geometry(request)?; emit_average_pool(request, emitter, geometry) }

fn emit_average_pool(
	request: &MaterializationRequest<'_>,
	emitter: &mut Emitter<'_>,
	geometry: PoolGeometry, ) -> OperationResult<()> {
	require_true(request, "window_indices_verified")?;
	let tree_lanes = prepared_tree_lanes(request)?;
	let divisor = exact_f32_extent(request, geometry.window_elements, "pool window")?;
	let values = input(request, "values")?;
	let window_indices = input(request, "window_indices")?;
	let pooled = output(request, "pooled")?;
	require_dtype(request, values, DType::F32, "values")?;
	require_dtype(request, window_indices, DType::I32, "window_indices")?;
	require_dtype(request, pooled, DType::F32, "pooled")?;
	require_shape(request, values, &[geometry.input_elements], "values")?;
	require_shape( request, window_indices, &[geometry.output_elements, geometry.window_elements],
		"window_indices",
	)?;
	require_shape(request, pooled, &[geometry.output_elements], "pooled")?;

	let windows = emitter.intermediate(DType::F32, window_indices.shape.clone())?; emitter.emit(
		vec![values.id, window_indices.id], vec![windows], PrimitiveKind::Gather(Gather { axis: 0,
			bounds: IndexBounds::Reject, }), )?; let sums = emitter.intermediate(DType::F32, pooled.shape.clone())?;
	emitter.emit( vec![windows], vec![sums], PrimitiveKind::Reduce(Reduce { operator: ReduceOperator::Sum,
			axes: pool_axis(request)?, keep_dimensions: false, result: ReduceResult::Value, tree_lanes, }), )?; emitter.emit(
		vec![sums], vec![pooled.id], PrimitiveKind::Elementwise(Elementwise { program: divide_by_program(request, divisor)?,
		}), ) }

fn emit_legacy_max_pool_1d(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi( request,
		&["values", "window_indices"],
		&["pooled", "winning_indices"],
		&[
			"batch",
			"window_length",
			"filters",
			"tree_lanes",
			"window_indices_verified",
		], )?;
	let batch = prepared_extent(request, "batch")?;
	let window = prepared_extent(request, "window_length")?;
	let filters = prepared_extent(request, "filters")?;
	let geometry = PoolGeometry { input_elements: checked_product( request, &[batch, window, filters],
			"maximum-pool input element count",
		)?, output_elements: checked_product( request, &[batch, filters],
			"maximum-pool output element count",
		)?, window_elements: window, input_width: window, kernel_width: window, };
	emit_max_pool(request, emitter, geometry, false) }

fn emit_channelwise_max_pool_1d(
	request: &MaterializationRequest<'_>,
	emitter: &mut Emitter<'_>,
) -> OperationResult<()> { require_exact_abi( request,
		&["values", "window_indices", "winner_bases"],
		&["pooled", "winning_indices"],
		&[
			"batch",
			"channelwise_nonoverlap",
			"channels",
			"input_length",
			"pool_size",
			"tail_window_repeats_last_coordinate",
			"tree_lanes",
			"window_indices_encode_channelwise_nonoverlap",
			"winner_bases_encode_channelwise_nonoverlap",
		], )?; for fact in [
		"channelwise_nonoverlap",
		"tail_window_repeats_last_coordinate",
		"window_indices_encode_channelwise_nonoverlap",
		"winner_bases_encode_channelwise_nonoverlap",
	] { require_true(request, fact)?; }
	let geometry = channelwise_pool_1d_geometry(request)?; let tree_lanes = prepared_tree_lanes(request)?;
	let values = input(request, "values")?;
	let window_indices = input(request, "window_indices")?;
	let winner_bases = input(request, "winner_bases")?;
	let pooled = output(request, "pooled")?;
	let winning_indices = output(request, "winning_indices")?;
	require_dtype(request, values, DType::F32, "values")?;
	require_dtype(request, window_indices, DType::I32, "window_indices")?;
	require_dtype(request, winner_bases, DType::I32, "winner_bases")?;
	require_dtype(request, pooled, DType::F32, "pooled")?;
	require_dtype(request, winning_indices, DType::I32, "winning_indices")?;
	require_shape(request, values, &[geometry.input_elements], "values")?;
	require_shape( request, window_indices, &[ geometry.batch, geometry.groups, geometry.channels, geometry.window_width,
		],
		"window_indices",
	)?; let output_shape = [geometry.batch, geometry.groups, geometry.channels]; for (name, tensor) in [
		("winner_bases", winner_bases),
		("pooled", pooled),
		("winning_indices", winning_indices),
	] { require_shape(request, tensor, &output_shape, name)?; }

	let windows = emitter.intermediate(DType::F32, window_indices.shape.clone())?; emitter.emit(
		vec![values.id, window_indices.id], vec![windows], PrimitiveKind::Gather(Gather { axis: 0,
			bounds: IndexBounds::Reject, }), )?; let maxima = emitter.intermediate(DType::F32, pooled.shape.clone())?;
	let local_indices = emitter.intermediate(DType::I32, winning_indices.shape.clone())?; emitter.emit( vec![windows],
		vec![maxima, local_indices], PrimitiveKind::Reduce(Reduce { operator: ReduceOperator::Maximum,
			axes: channelwise_pool_axis(request)?, keep_dimensions: false, result: ReduceResult::ValueAndIndex, tree_lanes, }),
	)?; emitter.emit( vec![maxima, local_indices, winner_bases.id], vec![pooled.id, winning_indices.id],
		PrimitiveKind::Elementwise(Elementwise { program: channelwise_pool_result_program(request, geometry.channels)?, }), )
}

fn emit_max_pool_2d(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi( request,
		&["values", "window_indices", "window_bases"],
		&["pooled", "winning_indices"],
		&[
			"batch",
			"channels",
			"input_height",
			"input_width",
			"kernel_height",
			"kernel_width",
			"stride_height",
			"stride_width",
			"tree_lanes",
			"window_indices_verified",
			"window_bases_verified",
		], )?;
	require_true(request, "window_bases_verified")?;
	let geometry = pooling_2d_geometry(request)?; emit_max_pool(request, emitter, geometry, true) }

fn emit_max_pool(
	request: &MaterializationRequest<'_>,
	emitter: &mut Emitter<'_>,
	geometry: PoolGeometry, two_dimensional: bool, ) -> OperationResult<()> {
	require_true(request, "window_indices_verified")?;
	let tree_lanes = prepared_tree_lanes(request)?;
	let values = input(request, "values")?;
	let window_indices = input(request, "window_indices")?;
	let pooled = output(request, "pooled")?;
	let winning_indices = output(request, "winning_indices")?;
	require_dtype(request, values, DType::F32, "values")?;
	require_dtype(request, window_indices, DType::I32, "window_indices")?;
	require_dtype(request, pooled, DType::F32, "pooled")?;
	require_dtype(request, winning_indices, DType::I32, "winning_indices")?;
	require_shape(request, values, &[geometry.input_elements], "values")?;
	require_shape( request, window_indices, &[geometry.output_elements, geometry.window_elements],
		"window_indices",
	)?;
	require_shape(request, pooled, &[geometry.output_elements], "pooled")?;
	require_shape( request, winning_indices, &[geometry.output_elements],
		"winning_indices",
	)?;

	let windows = emitter.intermediate(DType::F32, window_indices.shape.clone())?; emitter.emit(
		vec![values.id, window_indices.id], vec![windows], PrimitiveKind::Gather(Gather { axis: 0,
			bounds: IndexBounds::Reject, }), )?; let maxima = emitter.intermediate(DType::F32, pooled.shape.clone())?;
	let local_indices = emitter.intermediate(DType::I32, winning_indices.shape.clone())?; emitter.emit( vec![windows],
		vec![maxima, local_indices], PrimitiveKind::Reduce(Reduce { operator: ReduceOperator::Maximum,
			axes: pool_axis(request)?, keep_dimensions: false, result: ReduceResult::ValueAndIndex, tree_lanes, }), )?;
	match two_dimensional { true => {
			let bases = input(request, "window_bases")?;
			require_dtype(request, bases, DType::I32, "window_bases")?;
			require_shape(request, bases, &[geometry.output_elements], "window_bases")?;
			emitter.emit( vec![maxima, local_indices, bases.id], vec![pooled.id, winning_indices.id],
				PrimitiveKind::Elementwise(Elementwise { program: max_pool_2d_result_program(request, geometry)?, }), ) }
		false => { emitter.emit( vec![maxima, local_indices], vec![pooled.id, winning_indices.id],
				PrimitiveKind::Elementwise(Elementwise { program: typed_pair_identity_program(request)?, }), ) } } }

fn emit_average_pool_2d_backward(
	request: &MaterializationRequest<'_>,
	emitter: &mut Emitter<'_>,
) -> OperationResult<()> { require_exact_abi( request, &[
			"output_gradient",
			"gradient_indices",
			"destination_indices",
			"input_gradient_base",
		],
		&["input_gradient"],
		&[
			"batch",
			"channels",
			"input_height",
			"input_width",
			"kernel_height",
			"kernel_width",
			"stride_height",
			"stride_width",
			"gradient_indices_verified",
			"destination_indices_verified",
			"input_gradient_base_zero",
		], )?; let geometry = pooling_2d_geometry(request)?; let contributions = checked_product( request,
		&[geometry.output_elements, geometry.window_elements],
		"average-pool backward contribution count",
	)?; emit_average_backward( request, emitter, geometry.input_elements, geometry.output_elements, contributions,
		geometry.window_elements, false, ) }

fn emit_pool_gradient_expand(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi( request, &[
			"output_gradient",
			"gradient_indices",
			"destination_indices",
			"input_gradient_base",
		],
		&["input_gradient"],
		&[
			"batch",
			"window_length",
			"filters",
			"gradient_indices_verified",
			"destination_indices_verified",
			"destination_indices_unique",
			"input_gradient_base_zero",
		], )?;
	let batch = prepared_extent(request, "batch")?;
	let window = prepared_extent(request, "window_length")?;
	let filters = prepared_extent(request, "filters")?;
	let output_elements = checked_product( request, &[batch, filters],
		"pool-gradient source element count",
	)?; let input_elements = checked_product( request, &[batch, window, filters],
		"pool-gradient expanded element count",
	)?;
	require_true(request, "destination_indices_unique")?;
	emit_average_backward( request, emitter, input_elements, output_elements, input_elements, window, true, ) }

fn emit_average_backward(
	request: &MaterializationRequest<'_>,
	emitter: &mut Emitter<'_>,
	input_elements: u64, output_elements: u64, contributions: u64, divisor: u64, destinations_unique: bool,
) -> OperationResult<()> {
	require_true(request, "gradient_indices_verified")?;
	require_true(request, "destination_indices_verified")?;
	require_true(request, "input_gradient_base_zero")?;
	let divisor = exact_f32_extent(request, divisor, "pool window")?;
	let output_gradient = input(request, "output_gradient")?;
	let gradient_indices = input(request, "gradient_indices")?;
	let destination_indices = input(request, "destination_indices")?;
	let gradient_base = input(request, "input_gradient_base")?;
	let input_gradient = output(request, "input_gradient")?;
	require_dtype(request, output_gradient, DType::F32, "output_gradient")?;
	require_dtype(request, gradient_indices, DType::I32, "gradient_indices")?;
	require_dtype( request, destination_indices, DType::I32,
		"destination_indices",
	)?;
	require_dtype(request, gradient_base, DType::F32, "input_gradient_base")?;
	require_dtype(request, input_gradient, DType::F32, "input_gradient")?;
	require_shape( request, output_gradient, &[output_elements],
		"output_gradient",
	)?; require_shape( request, gradient_indices, &[contributions],
		"gradient_indices",
	)?; require_shape( request, destination_indices, &[contributions],
		"destination_indices",
	)?; require_shape( request, gradient_base, &[input_elements],
		"input_gradient_base",
	)?;
	require_same_tensor_contract(request, gradient_base, input_gradient, "input_gradient")?;

	let gathered = emitter.intermediate(DType::F32, gradient_indices.shape.clone())?; emitter.emit(
		vec![output_gradient.id, gradient_indices.id], vec![gathered], PrimitiveKind::Gather(Gather { axis: 0,
			bounds: IndexBounds::Reject, }), )?; let mapped = emitter.intermediate(DType::F32, gradient_indices.shape.clone())?;
	emitter.emit( vec![gathered], vec![mapped], PrimitiveKind::Elementwise(Elementwise {
			program: divide_by_program(request, divisor)?, }), )?; let conflict = match destinations_unique {
		true => ScatterConflict::UniqueIndices, false => { ScatterConflict::Atomic { operation: AtomicOperation::Add,
				ordering: AtomicOrdering::Relaxed, } } }; emitter.emit( vec![gradient_base.id, destination_indices.id, mapped],
		vec![input_gradient.id], PrimitiveKind::Scatter(Scatter { axis: 0, bounds: IndexBounds::Reject, conflict, }), ) }

fn emit_legacy_max_pool_1d_backward(
	request: &MaterializationRequest<'_>,
	emitter: &mut Emitter<'_>,
) -> OperationResult<()> { require_exact_abi( request, &[
			"output_gradient",
			"winning_indices",
			"gradient_indices",
			"destination_bases",
			"input_gradient_base",
		],
		&["input_gradient"],
		&[
			"batch",
			"window_length",
			"filters",
			"gradient_indices_verified",
			"destination_bases_verified",
			"destination_indices_unique",
			"input_gradient_base_zero",
		], )?;
	let batch = prepared_extent(request, "batch")?;
	let window = prepared_extent(request, "window_length")?;
	let filters = prepared_extent(request, "filters")?;
	let output_elements = checked_product( request, &[batch, filters],
		"maximum-pool backward output element count",
	)?; let input_elements = checked_product( request, &[batch, window, filters],
		"maximum-pool backward input element count",
	)?;
	require_true(request, "destination_indices_unique")?;
	emit_max_backward( request, emitter, input_elements, output_elements, filters, true, ) }

fn emit_channelwise_max_pool_1d_backward(
	request: &MaterializationRequest<'_>,
	emitter: &mut Emitter<'_>,
) -> OperationResult<()> { require_exact_abi( request, &[
			"output_gradient",
			"winning_indices",
			"gradient_batch_indices",
			"input_gradient_base",
		],
		&["input_gradient"],
		&[
			"batch",
			"channelwise_nonoverlap",
			"channels",
			"gradient_batch_indices_identity",
			"input_gradient_base_zero",
			"input_length",
			"pool_size",
			"winning_indices_from_matching_forward",
			"winning_indices_unique",
		], )?; for fact in [
		"channelwise_nonoverlap",
		"gradient_batch_indices_identity",
		"input_gradient_base_zero",
		"winning_indices_from_matching_forward",
		"winning_indices_unique",
	] { require_true(request, fact)?; }
	let geometry = channelwise_pool_1d_geometry(request)?;
	let output_gradient = input(request, "output_gradient")?;
	let winning_indices = input(request, "winning_indices")?;
	let gradient_batch_indices = input(request, "gradient_batch_indices")?;
	let gradient_base = input(request, "input_gradient_base")?;
	let input_gradient = output(request, "input_gradient")?;
	require_dtype(request, output_gradient, DType::F32, "output_gradient")?;
	require_dtype(request, winning_indices, DType::I32, "winning_indices")?;
	require_dtype( request, gradient_batch_indices, DType::I32,
		"gradient_batch_indices",
	)?;
	require_dtype(request, gradient_base, DType::F32, "input_gradient_base")?;
	require_dtype(request, input_gradient, DType::F32, "input_gradient")?;
	let output_shape = [geometry.batch, geometry.groups, geometry.channels];
	require_shape(request, output_gradient, &output_shape, "output_gradient")?;
	require_shape(request, winning_indices, &output_shape, "winning_indices")?;
	require_shape( request, gradient_batch_indices, &[geometry.batch],
		"gradient_batch_indices",
	)?; require_shape( request, gradient_base, &[geometry.input_elements],
		"input_gradient_base",
	)?;
	require_same_tensor_contract(request, gradient_base, input_gradient, "input_gradient")?;
	let gathered = emitter.intermediate(DType::F32, output_gradient.shape.clone())?; emitter.emit(
		vec![output_gradient.id, gradient_batch_indices.id], vec![gathered], PrimitiveKind::Gather(Gather { axis: 0,
			bounds: IndexBounds::Reject, }), )?; let mapped = emitter.intermediate(DType::F32, output_gradient.shape.clone())?;
	let destinations = emitter.intermediate(DType::I32, winning_indices.shape.clone())?; emitter.emit(
		vec![gathered, winning_indices.id], vec![mapped, destinations], PrimitiveKind::Elementwise(Elementwise {
			program: typed_pair_identity_program(request)?, }), )?; emitter.emit( vec![gradient_base.id, destinations, mapped],
		vec![input_gradient.id], PrimitiveKind::Scatter(Scatter { axis: 0, bounds: IndexBounds::Reject,
			conflict: ScatterConflict::UniqueIndices, }), ) }

fn emit_max_pool_2d_backward(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi( request, &[
			"output_gradient",
			"winning_indices",
			"gradient_indices",
			"plane_bases",
			"input_gradient_base",
		],
		&["input_gradient"],
		&[
			"batch",
			"channels",
			"input_height",
			"input_width",
			"output_height",
			"output_width",
			"gradient_indices_verified",
			"plane_bases_verified",
			"input_gradient_base_zero",
		], )?;
	let batch = prepared_extent(request, "batch")?;
	let channels = prepared_extent(request, "channels")?;
	let input_height = prepared_extent(request, "input_height")?;
	let input_width = prepared_extent(request, "input_width")?;
	let output_height = prepared_extent(request, "output_height")?;
	let output_width = prepared_extent(request, "output_width")?;
	let input_elements = checked_product( request, &[batch, channels, input_height, input_width],
		"maximum-pool backward input element count",
	)?; let output_elements = checked_product( request, &[batch, channels, output_height, output_width],
		"maximum-pool backward output element count",
	)?; emit_max_backward(request, emitter, input_elements, output_elements, 1, false) }

fn emit_max_backward(
	request: &MaterializationRequest<'_>,
	emitter: &mut Emitter<'_>,
	input_elements: u64, output_elements: u64, index_stride: u64, destinations_unique: bool, ) -> OperationResult<()> {
	require_true(request, "gradient_indices_verified")?;
	require_true(request, "input_gradient_base_zero")?;
	let bases_name = match destinations_unique {
		true => "destination_bases",
		false => "plane_bases",
	}; let bases_fact = match destinations_unique {
		true => "destination_bases_verified",
		false => "plane_bases_verified",
	}; require_true(request, bases_fact)?;
	let output_gradient = input(request, "output_gradient")?;
	let winning_indices = input(request, "winning_indices")?;
	let gradient_indices = input(request, "gradient_indices")?;
	let bases = input(request, bases_name)?;
	let gradient_base = input(request, "input_gradient_base")?;
	let input_gradient = output(request, "input_gradient")?;
	require_dtype(request, output_gradient, DType::F32, "output_gradient")?;
	require_dtype(request, winning_indices, DType::I32, "winning_indices")?;
	require_dtype(request, gradient_indices, DType::I32, "gradient_indices")?;
	require_dtype(request, bases, DType::I32, bases_name)?;
	require_dtype(request, gradient_base, DType::F32, "input_gradient_base")?;
	require_dtype(request, input_gradient, DType::F32, "input_gradient")?;
	for (name, tensor) in [
		("output_gradient", output_gradient),
		("winning_indices", winning_indices),
		("gradient_indices", gradient_indices),
		(bases_name, bases), ] { require_shape(request, tensor, &[output_elements], name)?; }
	require_shape( request, gradient_base, &[input_elements],
		"input_gradient_base",
	)?;
	require_same_tensor_contract(request, gradient_base, input_gradient, "input_gradient")?;

	let gathered = emitter.intermediate(DType::F32, output_gradient.shape.clone())?; emitter.emit(
		vec![output_gradient.id, gradient_indices.id], vec![gathered], PrimitiveKind::Gather(Gather { axis: 0,
			bounds: IndexBounds::Reject, }), )?; let mapped = emitter.intermediate(DType::F32, output_gradient.shape.clone())?;
	let destinations = emitter.intermediate(DType::I32, winning_indices.shape.clone())?; emitter.emit(
		vec![gathered, winning_indices.id, bases.id], vec![mapped, destinations], PrimitiveKind::Elementwise(Elementwise {
			program: max_pool_backward_program(request, index_stride)?, }), )?; let conflict = match destinations_unique {
		true => ScatterConflict::UniqueIndices, false => { ScatterConflict::Atomic { operation: AtomicOperation::Add,
				ordering: AtomicOrdering::Relaxed, } } }; emitter.emit( vec![gradient_base.id, destinations, mapped],
		vec![input_gradient.id], PrimitiveKind::Scatter(Scatter { axis: 0, bounds: IndexBounds::Reject, conflict, }), ) }

fn emit_upsample_nearest_2d(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(request, &["values", "source_indices"], &["upsampled"], &[
		"batch",
		"channels",
		"input_height",
		"input_width",
		"scale_height",
		"scale_width",
		"source_indices_verified",
	])?;
	let batch = prepared_extent(request, "batch")?;
	let channels = prepared_extent(request, "channels")?;
	let input_height = prepared_extent(request, "input_height")?;
	let input_width = prepared_extent(request, "input_width")?;
	let scale_height = prepared_extent(request, "scale_height")?;
	let scale_width = prepared_extent(request, "scale_width")?;
	let output_height = checked_product(request, &[input_height, scale_height], "upsampled height")?;
	let output_width = checked_product(request, &[input_width, scale_width], "upsampled width")?;
	let input_elements = checked_product( request, &[batch, channels, input_height, input_width],
		"upsample input element count",
	)?; let output_elements = checked_product( request, &[batch, channels, output_height, output_width],
		"upsample output element count",
	)?;
	require_true(request, "source_indices_verified")?;
	let values = input(request, "values")?;
	let indices = input(request, "source_indices")?;
	let upsampled = output(request, "upsampled")?;
	require_dtype(request, values, DType::F32, "values")?;
	require_dtype(request, indices, DType::I32, "source_indices")?;
	require_dtype(request, upsampled, DType::F32, "upsampled")?;
	require_shape(request, values, &[input_elements], "values")?;
	require_shape(request, indices, &[output_elements], "source_indices")?;
	require_shape(request, upsampled, &[output_elements], "upsampled")?;
	emit_checked_flat_gather(request, emitter, values, indices, upsampled) }

fn emit_checked_flat_gather(
	request: &MaterializationRequest<'_>,
	emitter: &mut Emitter<'_>,
	values: &recipe_language::Tensor, indices: &recipe_language::Tensor, result: &recipe_language::Tensor,
) -> OperationResult<()> { let gathered = emitter.intermediate(DType::F32, result.shape.clone())?; emitter.emit(
		vec![values.id, indices.id], vec![gathered], PrimitiveKind::Gather(Gather { axis: 0, bounds: IndexBounds::Reject, }),
	)?; emitter.emit( vec![gathered], vec![result.id], PrimitiveKind::Elementwise(Elementwise {
			program: identity_program(request.descriptor.id, DType::F32)?, }), ) }

fn convolution_2d_geometry(request: &MaterializationRequest<'_>, extended: bool) -> OperationResult<PoolGeometry> {
	let batch = prepared_extent(request, "batch")?;
	let channels = prepared_extent(request, "channels")?;
	let input_height = prepared_extent(request, "input_height")?;
	let input_width = prepared_extent(request, "input_width")?;
	let kernel_height = prepared_extent(request, "kernel_height")?;
	let kernel_width = prepared_extent(request, "kernel_width")?;
	let (stride_height, stride_width, padding_height, padding_width, dilation_height, dilation_width) = match extended
	{ true => { (
				prepared_extent(request, "stride_height")?,
				prepared_extent(request, "stride_width")?,
				prepared_u64(request.descriptor.id, request.parameters, "padding_height")?,
				prepared_u64(request.descriptor.id, request.parameters, "padding_width")?,
				prepared_extent(request, "dilation_height")?,
				prepared_extent(request, "dilation_width")?,
			) }
		false => (1, 1, 0, 0, 1, 1), };
	let effective_height = checked_effective_kernel(request, kernel_height, dilation_height, "height")?;
	let effective_width = checked_effective_kernel(request, kernel_width, dilation_width, "width")?;
	let padded_height = input_height .checked_add( padding_height .checked_mul(2)
				.ok_or_else(|| request_error(request, "padding_height * 2 overflowed u64"))?,
		)
		.ok_or_else(|| request_error(request, "padded input height overflowed u64"))?;
	let padded_width = input_width .checked_add( padding_width .checked_mul(2)
				.ok_or_else(|| request_error(request, "padding_width * 2 overflowed u64"))?,
		)
		.ok_or_else(|| request_error(request, "padded input width overflowed u64"))?;
	if effective_height > padded_height || effective_width > padded_width { return Err(operation_error(
			request.descriptor.id, OperationErrorKind::UnsupportedConcreteShape,
			"effective convolution kernel must fit the padded input",
		)); }
	let output_height = (padded_height - effective_height) / stride_height + 1;
	let output_width = (padded_width - effective_width) / stride_width + 1; let input_elements = checked_product( request,
		&[batch, channels, input_height, input_width],
		"image element count",
	)?; let output_elements = checked_product( request, &[batch, output_height, output_width],
		"patch count",
	)?; let window_elements = checked_product( request, &[channels, kernel_height, kernel_width],
		"patch size",
	)?; Ok(PoolGeometry { input_elements, output_elements, window_elements, input_width, kernel_width, }) }

fn pooling_2d_geometry(request: &MaterializationRequest<'_>) -> OperationResult<PoolGeometry> {
	let batch = prepared_extent(request, "batch")?;
	let channels = prepared_extent(request, "channels")?;
	let input_height = prepared_extent(request, "input_height")?;
	let input_width = prepared_extent(request, "input_width")?;
	let kernel_height = prepared_extent(request, "kernel_height")?;
	let kernel_width = prepared_extent(request, "kernel_width")?;
	let stride_height = prepared_extent(request, "stride_height")?;
	let stride_width = prepared_extent(request, "stride_width")?;
	if kernel_height > input_height || kernel_width > input_width { return Err(operation_error( request.descriptor.id,
			OperationErrorKind::UnsupportedConcreteShape,
			"pooling kernel must fit the input",
		)); }
	let output_height = (input_height - kernel_height) / stride_height + 1;
	let output_width = (input_width - kernel_width) / stride_width + 1; Ok(PoolGeometry { input_elements: checked_product(
			request, &[batch, channels, input_height, input_width],
			"pool input element count",
		)?, output_elements: checked_product( request, &[batch, channels, output_height, output_width],
			"pool output element count",
		)?, window_elements: checked_product( request, &[kernel_height, kernel_width],
			"pool window element count",
		)?, input_width, kernel_width, }) }

fn channelwise_pool_1d_geometry(request: &MaterializationRequest<'_>) -> OperationResult<ChannelwisePool1dGeometry> {
	let batch = prepared_extent(request, "batch")?;
	let input_length = prepared_extent(request, "input_length")?;
	let channels = prepared_extent(request, "channels")?;
	let pool_size = prepared_u64(request.descriptor.id, request.parameters, "pool_size")?;
	if pool_size == 0 { return Err(operation_error( request.descriptor.id, OperationErrorKind::UnsupportedConcreteShape,
			"pool_size must be positive",
		)); }
	let groups = input_length.div_ceil(pool_size); let window_width = input_length.min(pool_size);
	let input_elements = checked_product( request, &[batch, input_length, channels],
		"channelwise maximum-pool input element count",
	)?; let output_elements = checked_product( request, &[batch, groups, channels],
		"channelwise maximum-pool output element count",
	)?; checked_product( request, &[output_elements, window_width],
		"channelwise maximum-pool window-index element count",
	)?; Ok(ChannelwisePool1dGeometry { batch, channels, groups, window_width, input_elements, }) }

fn checked_effective_kernel(
	request: &MaterializationRequest<'_>,
	kernel: u64, dilation: u64, axis: &str, ) -> OperationResult<u64> { (kernel - 1) .checked_mul(dilation)
		.and_then(|value| value.checked_add(1))
		.ok_or_else(|| request_error(request, format!("effective {axis} kernel overflowed u64")))
}

fn prepared_extent(request: &MaterializationRequest<'_>, name: &str) -> OperationResult<u64> {
	let value = prepared_u64(request.descriptor.id, request.parameters, name)?; if value == 0 || value > MAX_I32_INDEX {
		return Err(operation_error( request.descriptor.id, OperationErrorKind::UnsupportedConcreteShape,
			format!("{name} must be in the legacy int32 extent range 1..={MAX_I32_INDEX}"),
		)); }
	Ok(value) }

fn checked_product(request: &MaterializationRequest<'_>, factors: &[u64], role: &str) -> OperationResult<u64> {
	let product = factors.iter().try_fold(1_u64, |product, factor| { product.checked_mul(*factor).ok_or_else(|| {
			operation_error( request.descriptor.id, OperationErrorKind::WorkspaceArithmeticOverflow,
				format!("{role} overflowed u64"),
			) }) })?; if product > MAX_I32_INDEX { return Err(operation_error( request.descriptor.id,
			OperationErrorKind::UnsupportedConcreteShape,
			format!("{role} must fit the legacy int32 linear-index domain"),
		)); }
	Ok(product) }

fn pool_axis(request: &MaterializationRequest<'_>) -> OperationResult<AxisSet> { AxisSet::new(vec![1]).map_err(|error| language_error(request.descriptor.id, error.to_string())) }

fn channelwise_pool_axis(request: &MaterializationRequest<'_>) -> OperationResult<AxisSet> { AxisSet::new(vec![3]).map_err(|error| language_error(request.descriptor.id, error.to_string())) }

fn divide_by_program(
	request: &MaterializationRequest<'_>,
	divisor: f32, ) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(request.descriptor.id)?;
	let value = scalar_input(request.descriptor.id, &mut builder, DType::F32)?;
	let divisor = scalar_f32(request.descriptor.id, &mut builder, divisor)?; let divided = scalar_binary(
		request.descriptor.id, &mut builder, ScalarOpcode::Divide, value, divisor, )?;
	scalar_finish(request.descriptor.id, builder, &[divided]) }

fn typed_pair_identity_program(request: &MaterializationRequest<'_>) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(request.descriptor.id)?;
	let value = scalar_input(request.descriptor.id, &mut builder, DType::F32)?;
	let index = scalar_input(request.descriptor.id, &mut builder, DType::I32)?;
	scalar_finish(request.descriptor.id, builder, &[value, index]) }

fn channelwise_pool_result_program(
	request: &MaterializationRequest<'_>,
	channels: u64, ) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(request.descriptor.id)?;
	let value = scalar_input(request.descriptor.id, &mut builder, DType::F32)?;
	let local = scalar_input(request.descriptor.id, &mut builder, DType::I32)?;
	let base = scalar_input(request.descriptor.id, &mut builder, DType::I32)?; let stride = builder
		.i32(i32_extent(request, channels, "channels")?)
		.map_err(|error| language_error(request.descriptor.id, error.to_string()))?; let offset = scalar_binary(
		request.descriptor.id, &mut builder, ScalarOpcode::Multiply, local, stride, )?; let global = scalar_binary(
		request.descriptor.id, &mut builder, ScalarOpcode::Add, base, offset, )?;
	scalar_finish(request.descriptor.id, builder, &[value, global]) }

fn max_pool_2d_result_program(
	request: &MaterializationRequest<'_>,
	geometry: PoolGeometry, ) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(request.descriptor.id)?;
	let value = scalar_input(request.descriptor.id, &mut builder, DType::F32)?;
	let local = scalar_input(request.descriptor.id, &mut builder, DType::I32)?;
	let base = scalar_input(request.descriptor.id, &mut builder, DType::I32)?; let kernel_width = builder
		.i32(i32_extent(request, geometry.kernel_width, "kernel_width")?)
		.map_err(|error| language_error(request.descriptor.id, error.to_string()))?; let input_width = builder
		.i32(i32_extent(request, geometry.input_width, "input_width")?)
		.map_err(|error| language_error(request.descriptor.id, error.to_string()))?; let row = scalar_binary(
		request.descriptor.id, &mut builder, ScalarOpcode::Divide, local, kernel_width, )?; let column = scalar_binary(
		request.descriptor.id, &mut builder, ScalarOpcode::Remainder, local, kernel_width, )?; let row_offset = scalar_binary(
		request.descriptor.id, &mut builder, ScalarOpcode::Multiply, row, input_width, )?; let offset = scalar_binary(
		request.descriptor.id, &mut builder, ScalarOpcode::Add, row_offset, column, )?; let global = scalar_binary(
		request.descriptor.id, &mut builder, ScalarOpcode::Add, base, offset, )?;
	scalar_finish(request.descriptor.id, builder, &[value, global]) }

fn max_pool_backward_program(
	request: &MaterializationRequest<'_>,
	index_stride: u64, ) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(request.descriptor.id)?;
	let gradient = scalar_input(request.descriptor.id, &mut builder, DType::F32)?;
	let winner = scalar_input(request.descriptor.id, &mut builder, DType::I32)?;
	let base = scalar_input(request.descriptor.id, &mut builder, DType::I32)?; let stride = builder
		.i32(i32_extent(request, index_stride, "index_stride")?)
		.map_err(|error| language_error(request.descriptor.id, error.to_string()))?; let offset = scalar_binary(
		request.descriptor.id, &mut builder, ScalarOpcode::Multiply, winner, stride, )?; let destination = scalar_binary(
		request.descriptor.id, &mut builder, ScalarOpcode::Add, base, offset, )?;
	scalar_finish(request.descriptor.id, builder, &[gradient, destination]) }

fn i32_extent(request: &MaterializationRequest<'_>, value: u64, name: &str) -> OperationResult<i32> {
	i32::try_from(value).map_err(|error| request_error(request, format!("{name} does not fit int32: {error}")))
}
