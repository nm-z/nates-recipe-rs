use std::error::Error;

use recipe_core::{ByteCount, DType, KernelTemplateId, ValueId};
use recipe_language::{AtomicOperation, AtomicOrdering, PrimitiveKind, ScatterConflict, Shape, Tensor};

use crate::{
	IdentityNamespace, MaterializationRequest, NamedTensor, OperationErrorKind, PreparedParameter,
	PreparedParameters, materialize_composition, operation_registry,
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

fn f32_parameter(value: f32) -> PreparedParameter {
	PreparedParameter::F32Bits(value.to_bits())
}

#[test]
fn materializes_head_merge_repeat_and_single_row_embedding_blend() -> TestResult {
	let heads = input_tensor(1, DType::F32, &[12])?;
	let merge_indices = input_tensor(2, DType::I32, &[12])?;
	let packed = output_tensor(3, DType::F32, &[12])?;
	let inputs = [
		NamedTensor::new("heads", &heads),
		NamedTensor::new("merge_indices", &merge_indices),
	];
	let outputs = [NamedTensor::new("packed", &packed)];
	let prepared = parameters(&[
		("seq", PreparedParameter::U64(2)),
		("n_heads", PreparedParameter::U64(2)),
		("head_dim", PreparedParameter::U64(3)),
		("merge_indices_verified", PreparedParameter::Bool(true)),
	]);
	let merge = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_mha_merge")?,
		&inputs,
		&outputs,
		"heads",
		&prepared,
		namespace(10_000),
		ByteCount::new(48),
	))?;
	assert_eq!(merge.workspace().bytes(), ByteCount::new(48));
	assert_eq!(merge.graph().nodes.len(), 2);
	merge.graph().validate()?;

	let values = input_tensor(11, DType::F32, &[4])?;
	let repeat_indices = input_tensor(12, DType::I32, &[12])?;
	let repeated = output_tensor(13, DType::F32, &[12])?;
	let inputs = [
		NamedTensor::new("values", &values),
		NamedTensor::new("repeat_indices", &repeat_indices),
	];
	let outputs = [NamedTensor::new("repeated", &repeated)];
	let prepared = parameters(&[
		("source_elements", PreparedParameter::U64(4)),
		("repeats", PreparedParameter::U64(3)),
		("repeat_indices_verified", PreparedParameter::Bool(true)),
	]);
	let repeated = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_repeat_rows")?,
		&inputs,
		&outputs,
		"values",
		&prepared,
		namespace(12_000),
		ByteCount::new(48),
	))?;
	assert_eq!(repeated.workspace().bytes(), ByteCount::new(48));
	repeated.graph().validate()?;

	let source_rows = input_tensor(21, DType::F32, &[5, 3])?;
	let row_indices = input_tensor(22, DType::I32, &[2])?;
	let weights = input_tensor(23, DType::F32, &[2, 1])?;
	let blended = output_tensor(24, DType::F32, &[2, 3])?;
	let inputs = [
		NamedTensor::new("source_rows", &source_rows),
		NamedTensor::new("row_indices", &row_indices),
		NamedTensor::new("weights", &weights),
	];
	let outputs = [NamedTensor::new("blended", &blended)];
	let prepared = parameters(&[
		("rows", PreparedParameter::U64(2)),
		("k", PreparedParameter::U64(1)),
		("source_row_count", PreparedParameter::U64(5)),
		("embedding_width", PreparedParameter::U64(3)),
		("scale", f32_parameter(0.5)),
		("row_indices_verified", PreparedParameter::Bool(true)),
	]);
	let blend = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_embed_blend")?,
		&inputs,
		&outputs,
		"source_rows",
		&prepared,
		namespace(14_000),
		ByteCount::new(24),
	))?;
	assert_eq!(blend.workspace().bytes(), ByteCount::new(24));
	assert_eq!(blend.graph().nodes.len(), 2);
	blend.graph().validate()?;
	Ok(())
}

#[test]
fn materializes_owned_causal_softmax_and_positional_encoding() -> TestResult {
	let values = input_tensor(1, DType::F32, &[3, 4])?;
	let causal_mask = input_tensor(2, DType::I32, &[3, 4])?;
	let softmax = output_tensor(3, DType::F32, &[3, 4])?;
	let inputs = [
		NamedTensor::new("values", &values),
		NamedTensor::new("causal_mask", &causal_mask),
	];
	let outputs = [NamedTensor::new("softmax", &softmax)];
	let prepared = parameters(&[
		("rows", PreparedParameter::U64(3)),
		("columns", PreparedParameter::U64(4)),
		("tree_lanes", PreparedParameter::U64(128)),
		("causal_mask_verified", PreparedParameter::Bool(true)),
	]);
	let softmax = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_causal_softmax_rows")?,
		&inputs,
		&outputs,
		"values",
		&prepared,
		namespace(20_000),
		ByteCount::new(216),
	))?;
	assert_eq!(softmax.workspace().bytes(), ByteCount::new(216));
	assert_eq!(softmax.stages().len(), 4);
	assert_eq!(softmax.graph().nodes.len(), 7);
	softmax.graph().validate()?;

	let angles = input_tensor(11, DType::F32, &[2, 4])?;
	let parity = input_tensor(12, DType::I32, &[2, 4])?;
	let encoding = output_tensor(13, DType::F32, &[2, 4])?;
	let inputs = [
		NamedTensor::new("angles", &angles),
		NamedTensor::new("channel_parity", &parity),
	];
	let outputs = [NamedTensor::new("encoding", &encoding)];
	let prepared = parameters(&[
		("seq", PreparedParameter::U64(2)),
		("dim", PreparedParameter::U64(4)),
		("angles_verified", PreparedParameter::Bool(true)),
	]);
	let positional = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_positional_encoding")?,
		&inputs,
		&outputs,
		"angles",
		&prepared,
		namespace(22_000),
		ByteCount::new(64),
	))?;
	assert_eq!(positional.workspace().bytes(), ByteCount::new(64));
	assert_eq!(positional.stages().len(), 1);
	assert_eq!(positional.graph().nodes.len(), 3);
	positional.graph().validate()?;
	Ok(())
}

#[test]
fn materializes_embedding_backward_with_explicit_atomic_policy() -> TestResult {
	let gradient = input_tensor(1, DType::F32, &[3, 2])?;
	let indices = input_tensor(2, DType::I32, &[3])?;
	let base = input_tensor(3, DType::F32, &[5, 2])?;
	let table = output_tensor(4, DType::F32, &[5, 2])?;
	let inputs = [
		NamedTensor::new("gradient", &gradient),
		NamedTensor::new("indices", &indices),
		NamedTensor::new("gradient_table_base", &base),
	];
	let outputs = [NamedTensor::new("gradient_table", &table)];
	let prepared = parameters(&[
		("rows", PreparedParameter::U64(3)),
		("columns", PreparedParameter::U64(2)),
		("vocabulary", PreparedParameter::U64(5)),
	]);
	let backward = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_embedding_backward")?,
		&inputs,
		&outputs,
		"gradient",
		&prepared,
		namespace(30_000),
		ByteCount::new(24),
	))?;
	assert_eq!(backward.workspace().bytes(), ByteCount::new(24));
	assert_eq!(backward.graph().nodes.len(), 2);
	match backward.graph().nodes[1].kernel.kind {
		PrimitiveKind::Scatter(scatter) => assert_eq!(
			scatter.conflict,
			ScatterConflict::Atomic {
				operation: AtomicOperation::Add,
				ordering: AtomicOrdering::Relaxed,
			}
		),
		_ => return Err("embedding backward did not end in scatter-add".into()),
	}
	backward.graph().validate()?;
	Ok(())
}

#[test]
fn every_single_tensor_rope_variant_uses_the_verified_rotation_table_abi() -> TestResult {
	for (ordinal, symbol) in [
		"gpu_rope",
		"gpu_rope_partial",
		"gpu_rope_partial_pos",
		"gpu_rope_partial_factors",
		"gpu_rope_partial_factors_pos",
	]
	.into_iter()
	.enumerate()
	{
		let is_full = symbol == "gpu_rope";
		let has_factors = symbol.contains("factors");
		let has_position = symbol.ends_with("_pos");
		let elements = if is_full { 8 } else { 18 };
		let values = input_tensor(1, DType::F32, &[elements])?;
		let partner_indices = input_tensor(2, DType::I32, &[elements])?;
		let cosines = input_tensor(3, DType::F32, &[elements])?;
		let signed_sines = input_tensor(4, DType::F32, &[elements])?;
		let factors = input_tensor(5, DType::F32, &[2])?;
		let rotated = output_tensor(6, DType::F32, &[elements])?;
		let mut inputs = vec![
			NamedTensor::new("values", &values),
			NamedTensor::new("partner_indices", &partner_indices),
			NamedTensor::new("cosines", &cosines),
			NamedTensor::new("signed_sines", &signed_sines),
		];
		if has_factors {
			inputs.push(NamedTensor::new("factors", &factors));
		}
		let outputs = [NamedTensor::new("rotated", &rotated)];
		let mut prepared = if is_full {
			parameters(&[
				("seq", PreparedParameter::U64(2)),
				("dim", PreparedParameter::U64(4)),
				("base", f32_parameter(10_000.0)),
				("rotation_tables_verified", PreparedParameter::Bool(true)),
			])
		} else {
			parameters(&[
				("rows", PreparedParameter::U64(3)),
				("head_dim", PreparedParameter::U64(6)),
				("rotary_dim", PreparedParameter::U64(4)),
				("heads_per_token", PreparedParameter::U64(2)),
				("theta", f32_parameter(10_000.0)),
				("rotation_tables_verified", PreparedParameter::Bool(true)),
			])
		};
		if has_position {
			prepared.insert("position_base".to_owned(), PreparedParameter::U64(7));
		}
		let materialized = materialize_composition(MaterializationRequest::new(
			operation_registry().resolve_unique(symbol)?,
			&inputs,
			&outputs,
			"values",
			&prepared,
			namespace(40_000 + u64::try_from(ordinal)? * 2_000),
			ByteCount::new(elements * 4),
		))?;
		assert_eq!(
			materialized.workspace().bytes(),
			ByteCount::new(elements * 4)
		);
		assert_eq!(materialized.graph().nodes.len(), 2);
		materialized.graph().validate()?;
	}
	Ok(())
}

#[test]
fn attention_batch_rejects_unverified_tables_and_unsupported_blend_width() -> TestResult {
	let values = input_tensor(1, DType::F32, &[8])?;
	let partner_indices = input_tensor(2, DType::I32, &[8])?;
	let cosines = input_tensor(3, DType::F32, &[8])?;
	let signed_sines = input_tensor(4, DType::F32, &[8])?;
	let rotated = output_tensor(5, DType::F32, &[8])?;
	let inputs = [
		NamedTensor::new("values", &values),
		NamedTensor::new("partner_indices", &partner_indices),
		NamedTensor::new("cosines", &cosines),
		NamedTensor::new("signed_sines", &signed_sines),
	];
	let outputs = [NamedTensor::new("rotated", &rotated)];
	let prepared = parameters(&[
		("seq", PreparedParameter::U64(2)),
		("dim", PreparedParameter::U64(4)),
		("base", f32_parameter(10_000.0)),
		("rotation_tables_verified", PreparedParameter::Bool(false)),
	]);
	let error = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_rope")?,
		&inputs,
		&outputs,
		"values",
		&prepared,
		namespace(60_000),
		ByteCount::new(32),
	))
	.expect_err("unverified RoPE tables were accepted");
	assert_eq!(
		error.kind,
		OperationErrorKind::InvalidMaterializationRequest
	);

	let source_rows = input_tensor(11, DType::F32, &[5, 3])?;
	let row_indices = input_tensor(12, DType::I32, &[2])?;
	let weights = input_tensor(13, DType::F32, &[2, 1])?;
	let blended = output_tensor(14, DType::F32, &[2, 3])?;
	let inputs = [
		NamedTensor::new("source_rows", &source_rows),
		NamedTensor::new("row_indices", &row_indices),
		NamedTensor::new("weights", &weights),
	];
	let outputs = [NamedTensor::new("blended", &blended)];
	let prepared = parameters(&[
		("rows", PreparedParameter::U64(2)),
		("k", PreparedParameter::U64(2)),
		("source_row_count", PreparedParameter::U64(5)),
		("embedding_width", PreparedParameter::U64(3)),
		("scale", f32_parameter(1.0)),
		("row_indices_verified", PreparedParameter::Bool(true)),
	]);
	let error = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_embed_blend")?,
		&inputs,
		&outputs,
		"source_rows",
		&prepared,
		namespace(62_000),
		ByteCount::new(24),
	))
	.expect_err("unsupported k > 1 embedding blend was accepted");
	assert_eq!(error.kind, OperationErrorKind::UnsupportedConcreteShape);
	Ok(())
}
