use recipe_core::{DType, ScalarOpcode};
use recipe_language::{
	AtomicOperation, AtomicOrdering, AxisSet, Elementwise, Gather, IndexBounds, PrimitiveKind, Reduce,
	ReduceOperator, ReduceResult, Scatter, ScatterConflict, Shape,
};
use recipe_math::MathFunction;

use super::{
	Emitter, FamilyDispatch, KernelEmission, MAX_I32_INDEX, MaterializationRequest, emit_checked_gather_identity,
	identity_program, input, language_error, operation_error, output, prepared_f32, prepared_u64, request_error,
	require_dtype, require_exact_abi, require_same_tensor_contract, require_shape, require_true, scalar_binary,
	scalar_builder, scalar_f32, scalar_finish, scalar_input, scalar_ternary, scalar_unary,
};
use crate::{OperationDescriptor, OperationErrorKind, OperationResult};

const OPERATIONS: &[(&str, &str)] = &[
	("gpu_causal_softmax_rows", "gpu-core/src/attention.rs:191"),
	("gpu_embed_blend", "gpu-core/src/infer_ops.rs:266"),
	("gpu_embedding_backward", "gpu-core/src/attention.rs:427"),
	("gpu_mha_merge", "gpu-core/src/attention.rs:227"),
	("gpu_mha_split", "gpu-core/src/attention.rs:202"),
	("gpu_positional_encoding", "gpu-core/src/attention.rs:276"),
	("gpu_repeat_rows", "gpu-core/src/kernels.rs:6598"),
	("gpu_rope", "gpu-core/src/attention.rs:252"),
	("gpu_rope_partial", "gpu-core/src/infer_ops.rs:154"),
	("gpu_rope_partial_factors", "gpu-core/src/infer_ops.rs:209"),
	(
		"gpu_rope_partial_factors_pos",
		"gpu-core/src/infer_ops.rs:237",
	),
	("gpu_rope_partial_pos", "gpu-core/src/infer_ops.rs:181"),
];

pub(super) fn supports(descriptor: OperationDescriptor) -> bool {
	OPERATIONS.contains(&(descriptor.symbol, descriptor.source))
}

pub(super) fn dispatch(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> FamilyDispatch {
	if !supports(request.descriptor) {
		return FamilyDispatch::NotOwned;
	}
	let result = match request.descriptor.symbol {
		"gpu_causal_softmax_rows" => emit_causal_softmax(request, emitter),
		"gpu_embed_blend" => emit_embedding_blend(request, emitter),
		"gpu_embedding_backward" => emit_embedding_backward(request, emitter),
		"gpu_mha_merge" => emit_mha_merge(request, emitter),
		"gpu_mha_split" => {
			emit_checked_gather_identity(request, emitter, "packed", "indices", "heads", "axis", None)
		}
		"gpu_positional_encoding" => emit_positional_encoding(request, emitter),
		"gpu_repeat_rows" => emit_repeat_rows(request, emitter),
		"gpu_rope"
		| "gpu_rope_partial"
		| "gpu_rope_partial_factors"
		| "gpu_rope_partial_factors_pos"
		| "gpu_rope_partial_pos" => emit_single_tensor_rope(request, emitter),
		symbol => {
			Err(operation_error(
				request.descriptor.id,
				OperationErrorKind::GraphMaterializationFailed,
				format!("attention/sequence/embedding dispatch is incomplete for {symbol}"),
			))
		}
	};
	FamilyDispatch::Owned(result)
}

fn emit_mha_merge(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(request, &["heads", "merge_indices"], &["packed"], &[
		"seq",
		"n_heads",
		"head_dim",
		"merge_indices_verified",
	])?;
	let heads = input(request, "heads")?;
	let indices = input(request, "merge_indices")?;
	let packed = output(request, "packed")?;
	let seq = prepared_dimension(request, "seq")?;
	let n_heads = prepared_dimension(request, "n_heads")?;
	let head_dim = prepared_dimension(request, "head_dim")?;
	let elements = checked_product(request, &[seq, n_heads, head_dim], "MHA element count")?;
	require_true(request, "merge_indices_verified")?;
	require_dtype(request, heads, DType::F32, "heads")?;
	require_dtype(request, indices, DType::I32, "merge_indices")?;
	require_dtype(request, packed, DType::F32, "packed")?;
	require_shape(request, heads, &[elements], "heads")?;
	require_shape(request, indices, &[elements], "merge_indices")?;
	require_shape(request, packed, &[elements], "packed")?;

	let gathered = emitter.intermediate(DType::F32, packed.shape.clone())?;
	emitter.emit(
		vec![heads.id, indices.id],
		vec![gathered],
		PrimitiveKind::Gather(Gather {
			axis: 0,
			bounds: IndexBounds::Reject,
		}),
	)?;
	emitter.emit(
		vec![gathered],
		vec![packed.id],
		PrimitiveKind::Elementwise(Elementwise {
			program: identity_program(request.descriptor.id, DType::F32)?,
		}),
	)
}

fn emit_repeat_rows(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(request, &["values", "repeat_indices"], &["repeated"], &[
		"source_elements",
		"repeats",
		"repeat_indices_verified",
	])?;
	let values = input(request, "values")?;
	let indices = input(request, "repeat_indices")?;
	let repeated = output(request, "repeated")?;
	let source_elements = prepared_dimension(request, "source_elements")?;
	let repeats = prepared_dimension(request, "repeats")?;
	let total = checked_product(
		request,
		&[source_elements, repeats],
		"repeated element count",
	)?;
	require_true(request, "repeat_indices_verified")?;
	require_dtype(request, values, DType::F32, "values")?;
	require_dtype(request, indices, DType::I32, "repeat_indices")?;
	require_dtype(request, repeated, DType::F32, "repeated")?;
	require_shape(request, values, &[source_elements], "values")?;
	require_shape(request, indices, &[total], "repeat_indices")?;
	require_shape(request, repeated, &[total], "repeated")?;

	let gathered = emitter.intermediate(DType::F32, repeated.shape.clone())?;
	emitter.emit(
		vec![values.id, indices.id],
		vec![gathered],
		PrimitiveKind::Gather(Gather {
			axis: 0,
			bounds: IndexBounds::Reject,
		}),
	)?;
	emitter.emit(
		vec![gathered],
		vec![repeated.id],
		PrimitiveKind::Elementwise(Elementwise {
			program: identity_program(request.descriptor.id, DType::F32)?,
		}),
	)
}

fn emit_embedding_blend(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(
		request,
		&["source_rows", "row_indices", "weights"],
		&["blended"],
		&[
			"rows",
			"k",
			"source_row_count",
			"embedding_width",
			"scale",
			"row_indices_verified",
		],
	)?;
	let source_rows = input(request, "source_rows")?;
	let row_indices = input(request, "row_indices")?;
	let weights = input(request, "weights")?;
	let blended = output(request, "blended")?;
	let rows = prepared_dimension(request, "rows")?;
	let k = prepared_dimension(request, "k")?;
	let source_row_count = prepared_dimension(request, "source_row_count")?;
	let embedding_width = prepared_dimension(request, "embedding_width")?;
	if k != 1 {
		return Err(operation_error(
			request.descriptor.id,
			OperationErrorKind::UnsupportedConcreteShape,
			"the concrete embedding-forward materializer currently supports the legacy k = 1 path only",
		));
	}
	let scale = prepared_f32(request, "scale")?;
	if !scale.is_finite() {
		return Err(request_error(request, "scale must be finite"));
	}
	require_true(request, "row_indices_verified")?;
	require_dtype(request, source_rows, DType::F32, "source_rows")?;
	require_dtype(request, row_indices, DType::I32, "row_indices")?;
	require_dtype(request, weights, DType::F32, "weights")?;
	require_dtype(request, blended, DType::F32, "blended")?;
	require_shape(
		request,
		source_rows,
		&[source_row_count, embedding_width],
		"source_rows",
	)?;
	require_shape(request, row_indices, &[rows], "row_indices")?;
	require_shape(request, weights, &[rows, 1], "weights")?;
	require_shape(request, blended, &[rows, embedding_width], "blended")?;

	let gathered = emitter.intermediate(DType::F32, blended.shape.clone())?;
	emitter.emit(
		vec![source_rows.id, row_indices.id],
		vec![gathered],
		PrimitiveKind::Gather(Gather {
			axis: 0,
			bounds: IndexBounds::Reject,
		}),
	)?;
	emitter.emit(
		vec![gathered, weights.id],
		vec![blended.id],
		PrimitiveKind::Elementwise(Elementwise {
			program: embedding_blend_program(request, scale)?,
		}),
	)
}

fn emit_embedding_backward(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(
		request,
		&["gradient", "indices", "gradient_table_base"],
		&["gradient_table"],
		&["rows", "columns", "vocabulary"],
	)?;
	let gradient = input(request, "gradient")?;
	let indices = input(request, "indices")?;
	let base = input(request, "gradient_table_base")?;
	let table = output(request, "gradient_table")?;
	let rows = prepared_dimension(request, "rows")?;
	let columns = prepared_dimension(request, "columns")?;
	let vocabulary = prepared_dimension(request, "vocabulary")?;
	checked_product(
		request,
		&[rows, columns],
		"embedding gradient element count",
	)?;
	checked_product(
		request,
		&[vocabulary, columns],
		"embedding table element count",
	)?;
	require_dtype(request, gradient, DType::F32, "gradient")?;
	require_dtype(request, indices, DType::I32, "indices")?;
	require_dtype(request, base, DType::F32, "gradient_table_base")?;
	require_dtype(request, table, DType::F32, "gradient_table")?;
	require_shape(request, gradient, &[rows, columns], "gradient")?;
	require_shape(request, indices, &[rows], "indices")?;
	require_shape(request, base, &[vocabulary, columns], "gradient_table_base")?;
	require_same_tensor_contract(request, base, table, "gradient_table")?;

	let mapped = emitter.intermediate(DType::F32, gradient.shape.clone())?;
	emitter.emit(
		vec![gradient.id],
		vec![mapped],
		PrimitiveKind::Elementwise(Elementwise {
			program: identity_program(request.descriptor.id, DType::F32)?,
		}),
	)?;
	emitter.emit(
		vec![base.id, indices.id, mapped],
		vec![table.id],
		PrimitiveKind::Scatter(Scatter {
			axis: 0,
			bounds: IndexBounds::Reject,
			conflict: ScatterConflict::Atomic {
				operation: AtomicOperation::Add,
				ordering: AtomicOrdering::Relaxed,
			},
		}),
	)
}

fn emit_positional_encoding(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(request, &["angles", "channel_parity"], &["encoding"], &[
		"seq",
		"dim",
		"angles_verified",
	])?;
	let angles = input(request, "angles")?;
	let parity = input(request, "channel_parity")?;
	let encoding = output(request, "encoding")?;
	let seq = prepared_dimension(request, "seq")?;
	let dim = prepared_dimension(request, "dim")?;
	require_true(request, "angles_verified")?;
	require_dtype(request, angles, DType::F32, "angles")?;
	require_dtype(request, parity, DType::I32, "channel_parity")?;
	require_dtype(request, encoding, DType::F32, "encoding")?;
	require_shape(request, angles, &[seq, dim], "angles")?;
	require_shape(request, parity, &[seq, dim], "channel_parity")?;
	require_shape(request, encoding, &[seq, dim], "encoding")?;

	let sine = emitter.intermediate(DType::F32, angles.shape.clone())?;
	let cosine = emitter.intermediate(DType::F32, angles.shape.clone())?;
	let sine_program = recipe_core::ScalarProgram::try_from(MathFunction::Sin)
		.map_err(|error| language_error(request.descriptor.id, error.to_string()))?;
	let cosine_program = recipe_core::ScalarProgram::try_from(MathFunction::Cos)
		.map_err(|error| language_error(request.descriptor.id, error.to_string()))?;
	emitter.emit_stage([
		KernelEmission {
			inputs: vec![angles.id],
			outputs: vec![sine],
			kind: PrimitiveKind::Elementwise(Elementwise {
				program: sine_program,
			}),
		},
		KernelEmission {
			inputs: vec![angles.id],
			outputs: vec![cosine],
			kind: PrimitiveKind::Elementwise(Elementwise {
				program: cosine_program,
			}),
		},
		KernelEmission {
			inputs: vec![sine, cosine, parity.id],
			outputs: vec![encoding.id],
			kind: PrimitiveKind::Elementwise(Elementwise {
				program: positional_select_program(request)?,
			}),
		},
	])
}

fn emit_causal_softmax(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(request, &["values", "causal_mask"], &["softmax"], &[
		"rows",
		"columns",
		"tree_lanes",
		"causal_mask_verified",
	])?;
	let values = input(request, "values")?;
	let mask = input(request, "causal_mask")?;
	let softmax = output(request, "softmax")?;
	let rows = prepared_dimension(request, "rows")?;
	let columns = prepared_dimension(request, "columns")?;
	let tree_lanes = prepared_tree_lanes(request)?;
	require_true(request, "causal_mask_verified")?;
	require_dtype(request, values, DType::F32, "values")?;
	require_dtype(request, mask, DType::I32, "causal_mask")?;
	require_dtype(request, softmax, DType::F32, "softmax")?;
	require_shape(request, values, &[rows, columns], "values")?;
	require_shape(request, mask, &[rows, columns], "causal_mask")?;
	require_same_tensor_contract(request, values, softmax, "softmax")?;

	let row_shape =
		Shape::new(vec![rows, 1]).map_err(|error| language_error(request.descriptor.id, error.to_string()))?;
	let masked = emitter.intermediate(DType::F32, values.shape.clone())?;
	let row_maximum = emitter.intermediate(DType::F32, row_shape.clone())?;
	emitter.emit_stage([
		KernelEmission {
			inputs: vec![values.id, mask.id],
			outputs: vec![masked],
			kind: PrimitiveKind::Elementwise(Elementwise {
				program: causal_max_mask_program(request)?,
			}),
		},
		KernelEmission {
			inputs: vec![masked],
			outputs: vec![row_maximum],
			kind: PrimitiveKind::Reduce(Reduce {
				operator: ReduceOperator::Maximum,
				axes: AxisSet::new(vec![1])
					.map_err(|error| language_error(request.descriptor.id, error.to_string()))?,
				keep_dimensions: true,
				result: ReduceResult::Value,
				tree_lanes,
			}),
		},
	])?;

	let safe_shift = emitter.intermediate(DType::F32, values.shape.clone())?;
	let raw_exponentials = emitter.intermediate(DType::F32, values.shape.clone())?;
	let exponentials = emitter.intermediate(DType::F32, values.shape.clone())?;
	let exponential_program = recipe_core::ScalarProgram::try_from(MathFunction::Exp)
		.map_err(|error| language_error(request.descriptor.id, error.to_string()))?;
	emitter.emit_stage([
		KernelEmission {
			inputs: vec![values.id, row_maximum, mask.id],
			outputs: vec![safe_shift],
			kind: PrimitiveKind::Elementwise(Elementwise {
				program: causal_safe_shift_program(request)?,
			}),
		},
		KernelEmission {
			inputs: vec![safe_shift],
			outputs: vec![raw_exponentials],
			kind: PrimitiveKind::Elementwise(Elementwise {
				program: exponential_program,
			}),
		},
		KernelEmission {
			inputs: vec![raw_exponentials, mask.id],
			outputs: vec![exponentials],
			kind: PrimitiveKind::Elementwise(Elementwise {
				program: mask_to_zero_program(request)?,
			}),
		},
	])?;

	let row_sums = emitter.intermediate(DType::F32, row_shape)?;
	emitter.emit(
		vec![exponentials],
		vec![row_sums],
		PrimitiveKind::Reduce(Reduce {
			operator: ReduceOperator::Sum,
			axes: AxisSet::new(vec![1])
				.map_err(|error| language_error(request.descriptor.id, error.to_string()))?,
			keep_dimensions: true,
			result: ReduceResult::Value,
			tree_lanes,
		}),
	)?;
	emitter.emit(
		vec![exponentials, row_sums],
		vec![softmax.id],
		PrimitiveKind::Elementwise(Elementwise {
			program: checked_divide_program(request)?,
		}),
	)
}

fn emit_single_tensor_rope(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	let symbol = request.descriptor.symbol;
	let has_factors = matches!(
		symbol,
		"gpu_rope_partial_factors" | "gpu_rope_partial_factors_pos"
	);
	let has_position_base = matches!(
		symbol,
		"gpu_rope_partial_pos" | "gpu_rope_partial_factors_pos"
	);
	let input_names: &[&str] = if has_factors {
		&[
			"values",
			"partner_indices",
			"cosines",
			"signed_sines",
			"factors",
		]
	} else {
		&["values", "partner_indices", "cosines", "signed_sines"]
	};
	let full_parameters = ["seq", "dim", "base", "rotation_tables_verified"];
	let partial_parameters = [
		"rows",
		"head_dim",
		"rotary_dim",
		"heads_per_token",
		"theta",
		"rotation_tables_verified",
	];
	let partial_position_parameters = [
		"rows",
		"head_dim",
		"rotary_dim",
		"heads_per_token",
		"theta",
		"position_base",
		"rotation_tables_verified",
	];
	let parameter_names = if symbol == "gpu_rope" {
		full_parameters.as_slice()
	} else if has_position_base {
		partial_position_parameters.as_slice()
	} else {
		partial_parameters.as_slice()
	};
	require_exact_abi(request, input_names, &["rotated"], parameter_names)?;

	let (elements, rotary_half) = if symbol == "gpu_rope" {
		let seq = prepared_dimension(request, "seq")?;
		let dim = prepared_even_dimension(request, "dim")?;
		let base = prepared_f32(request, "base")?;
		require_finite_positive(request, "base", base)?;
		(
			checked_product(request, &[seq, dim], "RoPE element count")?,
			dim / 2,
		)
	} else {
		let rows = prepared_dimension(request, "rows")?;
		let head_dim = prepared_dimension(request, "head_dim")?;
		let rotary_dim = prepared_even_dimension(request, "rotary_dim")?;
		if rotary_dim > head_dim {
			return Err(operation_error(
				request.descriptor.id,
				OperationErrorKind::UnsupportedConcreteShape,
				"rotary_dim must not exceed head_dim",
			));
		}
		let heads_per_token = prepared_dimension(request, "heads_per_token")?;
		let theta = prepared_f32(request, "theta")?;
		require_finite_positive(request, "theta", theta)?;
		if has_position_base {
			let position_base = prepared_u64(request.descriptor.id, request.parameters, "position_base")?;
			let final_position = position_base
				.checked_add((rows - 1) / heads_per_token)
				.ok_or_else(|| {
					operation_error(
						request.descriptor.id,
						OperationErrorKind::UnsupportedConcreteShape,
						"partial RoPE final position overflowed u64",
					)
				})?;
			if final_position > MAX_I32_INDEX {
				return Err(operation_error(
					request.descriptor.id,
					OperationErrorKind::UnsupportedConcreteShape,
					"the final partial RoPE position must fit the legacy int32 coordinate domain",
				));
			}
		}
		(
			checked_product(request, &[rows, head_dim], "partial RoPE element count")?,
			rotary_dim / 2,
		)
	};
	require_true(request, "rotation_tables_verified")?;

	let values = input(request, "values")?;
	let partner_indices = input(request, "partner_indices")?;
	let cosines = input(request, "cosines")?;
	let signed_sines = input(request, "signed_sines")?;
	let rotated = output(request, "rotated")?;
	require_dtype(request, values, DType::F32, "values")?;
	require_dtype(request, partner_indices, DType::I32, "partner_indices")?;
	require_dtype(request, cosines, DType::F32, "cosines")?;
	require_dtype(request, signed_sines, DType::F32, "signed_sines")?;
	require_dtype(request, rotated, DType::F32, "rotated")?;
	for (name, tensor) in [
		("values", values),
		("partner_indices", partner_indices),
		("cosines", cosines),
		("signed_sines", signed_sines),
		("rotated", rotated),
	] {
		require_shape(request, tensor, &[elements], name)?;
	}
	if has_factors {
		let factors = input(request, "factors")?;
		require_dtype(request, factors, DType::F32, "factors")?;
		require_shape(request, factors, &[rotary_half], "factors")?;
	}

	let partners = emitter.intermediate(DType::F32, values.shape.clone())?;
	emitter.emit(
		vec![values.id, partner_indices.id],
		vec![partners],
		PrimitiveKind::Gather(Gather {
			axis: 0,
			bounds: IndexBounds::Reject,
		}),
	)?;
	emitter.emit(
		vec![values.id, partners, cosines.id, signed_sines.id],
		vec![rotated.id],
		PrimitiveKind::Elementwise(Elementwise {
			program: rotation_program(request)?,
		}),
	)
}

fn prepared_dimension(request: &MaterializationRequest<'_>, name: &str) -> OperationResult<u64> {
	let value = prepared_u64(request.descriptor.id, request.parameters, name)?;
	if value == 0 || value > MAX_I32_INDEX {
		return Err(operation_error(
			request.descriptor.id,
			OperationErrorKind::UnsupportedConcreteShape,
			format!(
				"{name} must be in the legacy int32 extent range 1..={}",
				i32::MAX
			),
		));
	}
	Ok(value)
}

fn prepared_even_dimension(request: &MaterializationRequest<'_>, name: &str) -> OperationResult<u64> {
	let value = prepared_dimension(request, name)?;
	if !value.is_multiple_of(2) {
		return Err(operation_error(
			request.descriptor.id,
			OperationErrorKind::UnsupportedConcreteShape,
			format!("{name} must be even"),
		));
	}
	Ok(value)
}

fn prepared_tree_lanes(request: &MaterializationRequest<'_>) -> OperationResult<u32> {
	let value = prepared_u64(request.descriptor.id, request.parameters, "tree_lanes")?;
	let lanes = u32::try_from(value)
		.map_err(|error| request_error(request, format!("tree_lanes does not fit u32: {error}")))?;
	if lanes == 0 || lanes > 1024 || !lanes.is_power_of_two() {
		return Err(request_error(
			request,
			"tree_lanes must be a power of two in 1..=1024",
		));
	}
	Ok(lanes)
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
	if product > MAX_I32_INDEX {
		return Err(operation_error(
			request.descriptor.id,
			OperationErrorKind::UnsupportedConcreteShape,
			format!("{role} must fit the legacy int32 linear-index domain"),
		));
	}
	Ok(product)
}

fn require_finite_positive(request: &MaterializationRequest<'_>, name: &str, value: f32) -> OperationResult<()> {
	if value.is_finite() && value > 0.0 {
		Ok(())
	} else {
		Err(request_error(
			request,
			format!("{name} must be finite and positive"),
		))
	}
}

fn embedding_blend_program(
	request: &MaterializationRequest<'_>,
	scale: f32,
) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(request.descriptor.id)?;
	let value = scalar_input(request.descriptor.id, &mut builder, DType::F32)?;
	let weight = scalar_input(request.descriptor.id, &mut builder, DType::F32)?;
	let weighted = scalar_binary(
		request.descriptor.id,
		&mut builder,
		ScalarOpcode::Multiply,
		value,
		weight,
	)?;
	let scale = scalar_f32(request.descriptor.id, &mut builder, scale)?;
	let blended = scalar_binary(
		request.descriptor.id,
		&mut builder,
		ScalarOpcode::Multiply,
		weighted,
		scale,
	)?;
	scalar_finish(request.descriptor.id, builder, &[blended])
}

fn positional_select_program(request: &MaterializationRequest<'_>) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(request.descriptor.id)?;
	let sine = scalar_input(request.descriptor.id, &mut builder, DType::F32)?;
	let cosine = scalar_input(request.descriptor.id, &mut builder, DType::F32)?;
	let parity = scalar_input(request.descriptor.id, &mut builder, DType::I32)?;
	let zero = builder
		.i32(0)
		.map_err(|error| language_error(request.descriptor.id, error.to_string()))?;
	let one = builder
		.i32(1)
		.map_err(|error| language_error(request.descriptor.id, error.to_string()))?;
	let is_zero = scalar_binary(
		request.descriptor.id,
		&mut builder,
		ScalarOpcode::Equal,
		parity,
		zero,
	)?;
	let is_one = scalar_binary(
		request.descriptor.id,
		&mut builder,
		ScalarOpcode::Equal,
		parity,
		one,
	)?;
	let valid = scalar_binary(
		request.descriptor.id,
		&mut builder,
		ScalarOpcode::BitOr,
		is_zero,
		is_one,
	)?;
	scalar_unary(
		request.descriptor.id,
		&mut builder,
		ScalarOpcode::Require,
		valid,
	)?;
	let selected = scalar_ternary(
		request.descriptor.id,
		&mut builder,
		ScalarOpcode::Select,
		parity,
		cosine,
		sine,
	)?;
	scalar_finish(request.descriptor.id, builder, &[selected])
}

fn causal_max_mask_program(request: &MaterializationRequest<'_>) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(request.descriptor.id)?;
	let value = scalar_input(request.descriptor.id, &mut builder, DType::F32)?;
	let mask = scalar_input(request.descriptor.id, &mut builder, DType::I32)?;
	let floor = scalar_f32(request.descriptor.id, &mut builder, -1.0e30)?;
	let selected = scalar_ternary(
		request.descriptor.id,
		&mut builder,
		ScalarOpcode::Select,
		mask,
		value,
		floor,
	)?;
	scalar_finish(request.descriptor.id, builder, &[selected])
}

fn causal_safe_shift_program(request: &MaterializationRequest<'_>) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(request.descriptor.id)?;
	let value = scalar_input(request.descriptor.id, &mut builder, DType::F32)?;
	let maximum = scalar_input(request.descriptor.id, &mut builder, DType::F32)?;
	let mask = scalar_input(request.descriptor.id, &mut builder, DType::I32)?;
	let shifted = scalar_binary(
		request.descriptor.id,
		&mut builder,
		ScalarOpcode::Subtract,
		value,
		maximum,
	)?;
	let zero = scalar_f32(request.descriptor.id, &mut builder, 0.0)?;
	let safe = scalar_ternary(
		request.descriptor.id,
		&mut builder,
		ScalarOpcode::Select,
		mask,
		shifted,
		zero,
	)?;
	scalar_finish(request.descriptor.id, builder, &[safe])
}

fn mask_to_zero_program(request: &MaterializationRequest<'_>) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(request.descriptor.id)?;
	let value = scalar_input(request.descriptor.id, &mut builder, DType::F32)?;
	let mask = scalar_input(request.descriptor.id, &mut builder, DType::I32)?;
	let zero = scalar_f32(request.descriptor.id, &mut builder, 0.0)?;
	let selected = scalar_ternary(
		request.descriptor.id,
		&mut builder,
		ScalarOpcode::Select,
		mask,
		value,
		zero,
	)?;
	scalar_finish(request.descriptor.id, builder, &[selected])
}

fn checked_divide_program(request: &MaterializationRequest<'_>) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(request.descriptor.id)?;
	let numerator = scalar_input(request.descriptor.id, &mut builder, DType::F32)?;
	let denominator = scalar_input(request.descriptor.id, &mut builder, DType::F32)?;
	let zero = scalar_f32(request.descriptor.id, &mut builder, 0.0)?;
	let positive = scalar_binary(
		request.descriptor.id,
		&mut builder,
		ScalarOpcode::GreaterThan,
		denominator,
		zero,
	)?;
	scalar_unary(
		request.descriptor.id,
		&mut builder,
		ScalarOpcode::Require,
		positive,
	)?;
	let quotient = scalar_binary(
		request.descriptor.id,
		&mut builder,
		ScalarOpcode::Divide,
		numerator,
		denominator,
	)?;
	scalar_finish(request.descriptor.id, builder, &[quotient])
}

fn rotation_program(request: &MaterializationRequest<'_>) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(request.descriptor.id)?;
	let value = scalar_input(request.descriptor.id, &mut builder, DType::F32)?;
	let partner = scalar_input(request.descriptor.id, &mut builder, DType::F32)?;
	let cosine = scalar_input(request.descriptor.id, &mut builder, DType::F32)?;
	let signed_sine = scalar_input(request.descriptor.id, &mut builder, DType::F32)?;
	let cosine_term = scalar_binary(
		request.descriptor.id,
		&mut builder,
		ScalarOpcode::Multiply,
		value,
		cosine,
	)?;
	let sine_term = scalar_binary(
		request.descriptor.id,
		&mut builder,
		ScalarOpcode::Multiply,
		partner,
		signed_sine,
	)?;
	let rotated = scalar_binary(
		request.descriptor.id,
		&mut builder,
		ScalarOpcode::Add,
		cosine_term,
		sine_term,
	)?;
	scalar_finish(request.descriptor.id, builder, &[rotated])
}
