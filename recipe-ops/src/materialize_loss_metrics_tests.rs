use std::collections::BTreeSet;
use std::error::Error;

use recipe_core::{ByteCount, DType, KernelTemplateId, ScalarOpcode, ValueId};
use recipe_language::{PrimitiveKind, Reduce, ReduceOperator, ReduceResult, Shape, Tensor};

use crate::{
	DeterminismContract, IdentityNamespace, MaterializationRequest, MaterializedComposition, NamedTensor,
	OperationErrorKind, PreparedParameter, PreparedParameters, materialize_composition, operation_registry,
	remaining_composition_manifest,
};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

const MATERIALIZED_SYMBOLS: &[&str] = &[
	"gpu_accuracy",
	"gpu_accuracy_into",
	"gpu_argmax_accuracy_into",
	"gpu_contrastive_loss",
	"gpu_cosine_embedding_loss",
	"gpu_has_nan",
	"gpu_hinge_loss",
	"gpu_isfinite_all",
	"gpu_kl_div_loss",
	"gpu_mean_all",
	"gpu_mse_into",
	"gpu_reduce_mean_cols",
	"gpu_reduce_var_cols",
	"gpu_ss_res_into",
	"gpu_triplet_loss",
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

fn tree_parameters(lanes: u64) -> PreparedParameters {
	parameters(&[("tree_lanes", PreparedParameter::U64(lanes))])
}

fn margin_parameters(margin: f32, lanes: u64) -> PreparedParameters {
	parameters(&[
		("margin", PreparedParameter::F32Bits(margin.to_bits())),
		("tree_lanes", PreparedParameter::U64(lanes)),
	])
}

fn reductions(materialized: &MaterializedComposition) -> Vec<&Reduce> {
	materialized
		.graph()
		.nodes
		.iter()
		.filter_map(|node| match &node.kernel.kind {
			PrimitiveKind::Reduce(reduction) => Some(reduction),
			PrimitiveKind::Elementwise(_)
			| PrimitiveKind::Scan(_)
			| PrimitiveKind::Contraction(_)
			| PrimitiveKind::Gather(_)
			| PrimitiveKind::Scatter(_)
			| PrimitiveKind::Histogram(_)
			| PrimitiveKind::Sort(_)
			| PrimitiveKind::IndexMap(_)
			| PrimitiveKind::Random(_) => None,
		})
		.collect()
}

fn assert_fragment(
	materialized: &MaterializedComposition,
	workspace_bytes: u64,
	stage_count: usize,
	tree_lanes: Option<u32>,
) -> TestResult {
	materialized.graph().validate()?;
	assert_eq!(
		materialized.workspace().bytes(),
		ByteCount::new(workspace_bytes)
	);
	assert_eq!(materialized.stages().len(), stage_count);
	assert!(
		materialized
			.stages()
			.iter()
			.all(|stage| !stage.kernels().is_empty())
	);
	match tree_lanes {
		Some(expected) => {
			let reductions = reductions(materialized);
			assert!(!reductions.is_empty());
			assert!(
				reductions
					.iter()
					.all(|reduction| reduction.tree_lanes == expected)
			);
		}
		None => assert!(reductions(materialized).is_empty()),
	}
	Ok(())
}

fn opcodes(materialized: &MaterializedComposition) -> Vec<ScalarOpcode> {
	let mut opcodes = Vec::new();
	for node in &materialized.graph().nodes {
		match &node.kernel.kind {
			PrimitiveKind::Elementwise(elementwise) => {
				opcodes.extend(
					elementwise
						.program
						.instructions
						.iter()
						.map(|instruction| instruction.opcode),
				);
			}
			PrimitiveKind::Reduce(_)
			| PrimitiveKind::Scan(_)
			| PrimitiveKind::Contraction(_)
			| PrimitiveKind::Gather(_)
			| PrimitiveKind::Scatter(_)
			| PrimitiveKind::Histogram(_)
			| PrimitiveKind::Sort(_)
			| PrimitiveKind::IndexMap(_)
			| PrimitiveKind::Random(_) => continue,
		}
	}
	opcodes
}

#[test]
fn owns_exactly_fifteen_source_qualified_loss_metric_descriptors() -> TestResult {
	let remaining = remaining_composition_manifest()
		.into_iter()
		.map(|entry| (entry.symbol(), entry.source()))
		.collect::<BTreeSet<_>>();
	for symbol in MATERIALIZED_SYMBOLS {
		let descriptor = operation_registry().resolve_unique(symbol)?;
		assert_eq!(
			descriptor.determinism,
			DeterminismContract::FixedPrimitiveOrder
		);
		assert!(!remaining.contains(&(descriptor.symbol, descriptor.source)));
	}

	let scores = input_tensor(1, DType::F32, &[4])?;
	let labels = input_tensor(2, DType::F32, &[4])?;
	let losses = output_tensor(3, DType::F32, &[4])?;
	let gradients = output_tensor(4, DType::F32, &[4])?;
	let inputs = [
		NamedTensor::new("scores", &scores),
		NamedTensor::new("labels", &labels),
	];
	let outputs = [
		NamedTensor::new("losses", &losses),
		NamedTensor::new("gradients", &gradients),
	];
	let mut wrong_source = operation_registry().resolve_unique("gpu_hinge_loss")?;
	wrong_source.source = "gpu-core/src/kernels.rs:246";
	let error = materialize_composition(MaterializationRequest::new(
		wrong_source,
		&inputs,
		&outputs,
		"scores",
		&PreparedParameters::new(),
		namespace(10_000),
		ByteCount::new(0),
	))
	.expect_err("a symbol-only loss match must fail closed");
	assert_eq!(error.kind, OperationErrorKind::MissingConcreteFormula);
	Ok(())
}

#[test]
fn materializes_accuracy_and_finite_metrics_with_fixed_reductions() -> TestResult {
	let predictions = input_tensor(1, DType::F32, &[6])?;
	let targets = input_tensor(2, DType::F32, &[6])?;
	let accuracy = output_tensor(3, DType::F32, &[1])?;
	let inputs = [
		NamedTensor::new("predictions", &predictions),
		NamedTensor::new("targets", &targets),
	];
	let outputs = [NamedTensor::new("accuracy", &accuracy)];
	let prepared = tree_parameters(64);
	let binary = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_accuracy_into")?,
		&inputs,
		&outputs,
		"predictions",
		&prepared,
		namespace(20_000),
		ByteCount::new(28),
	))?;
	assert_fragment(&binary, 28, 3, Some(64))?;
	let binary_reductions = reductions(&binary);
	assert_eq!(binary_reductions[0].operator, ReduceOperator::Sum);
	assert_eq!(binary_reductions[0].axes.as_slice(), [0]);
	assert!(opcodes(&binary).contains(&ScalarOpcode::GreaterThanOrEqual));
	assert!(opcodes(&binary).contains(&ScalarOpcode::ConvertI32ToF32));

	let predictions = input_tensor(11, DType::F32, &[3, 4])?;
	let targets = input_tensor(12, DType::I32, &[3])?;
	let correct_count = output_tensor(13, DType::F32, &[1])?;
	let inputs = [
		NamedTensor::new("predictions", &predictions),
		NamedTensor::new("targets", &targets),
	];
	let outputs = [NamedTensor::new("correct_count", &correct_count)];
	let multiclass = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_accuracy")?,
		&inputs,
		&outputs,
		"predictions",
		&prepared,
		namespace(22_000),
		ByteCount::new(24),
	))?;
	assert_fragment(&multiclass, 24, 3, Some(64))?;
	let multiclass_reductions = reductions(&multiclass);
	assert_eq!(multiclass_reductions[0].operator, ReduceOperator::Maximum);
	assert_eq!(multiclass_reductions[0].result, ReduceResult::Index);
	assert_eq!(multiclass_reductions[0].axes.as_slice(), [1]);
	assert_eq!(multiclass_reductions[1].operator, ReduceOperator::Sum);
	assert_eq!(multiclass_reductions[1].axes.as_slice(), [0]);
	assert!(
		multiclass
			.graph()
			.nodes
			.iter()
			.flat_map(|node| match &node.kernel.kind {
				PrimitiveKind::Elementwise(elementwise) => elementwise.program.instructions.iter(),
				PrimitiveKind::Reduce(_)
				| PrimitiveKind::Scan(_)
				| PrimitiveKind::Contraction(_)
				| PrimitiveKind::Gather(_)
				| PrimitiveKind::Scatter(_)
				| PrimitiveKind::Histogram(_)
				| PrimitiveKind::Sort(_)
				| PrimitiveKind::IndexMap(_)
				| PrimitiveKind::Random(_) => [].iter(),
			})
			.filter(|instruction| instruction.opcode == ScalarOpcode::Require)
			.count() >= 2
	);

	let dense_targets = input_tensor(22, DType::F32, &[3, 4])?;
	let dense_accuracy = output_tensor(23, DType::F32, &[1])?;
	let dense_inputs = [
		NamedTensor::new("predictions", &predictions),
		NamedTensor::new("targets", &dense_targets),
	];
	let dense_outputs = [NamedTensor::new("accuracy", &dense_accuracy)];
	let dense = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_argmax_accuracy_into")?,
		&dense_inputs,
		&dense_outputs,
		"predictions",
		&prepared,
		namespace(24_000),
		ByteCount::new(40),
	))?;
	assert_fragment(&dense, 40, 5, Some(64))?;
	let dense_reductions = reductions(&dense);
	assert_eq!(dense_reductions.len(), 3);
	assert!(
		dense_reductions[..2]
			.iter()
			.all(|reduction| reduction.result == ReduceResult::Index)
	);

	for (index, (symbol, output_name, operator)) in [
		("gpu_has_nan", "has_nan", ReduceOperator::Any),
		("gpu_isfinite_all", "all_finite", ReduceOperator::All),
	]
	.into_iter()
	.enumerate()
	{
		let values = input_tensor(31, DType::F32, &[2, 3])?;
		let flag = output_tensor(32, DType::I32, &[1])?;
		let finite_inputs = [NamedTensor::new("values", &values)];
		let finite_outputs = [NamedTensor::new(output_name, &flag)];
		let materialized = materialize_composition(MaterializationRequest::new(
			operation_registry().resolve_unique(symbol)?,
			&finite_inputs,
			&finite_outputs,
			"values",
			&prepared,
			namespace(26_000 + u64::try_from(index)? * 2_000),
			ByteCount::new(24),
		))?;
		assert_fragment(&materialized, 24, 2, Some(64))?;
		assert_eq!(reductions(&materialized)[0].operator, operator);
	}
	Ok(())
}

#[test]
fn materializes_scalar_losses_and_global_statistics_with_exact_workspace() -> TestResult {
	let scores = input_tensor(1, DType::F32, &[2, 3])?;
	let labels = input_tensor(2, DType::F32, &[2, 3])?;
	let losses = output_tensor(3, DType::F32, &[2, 3])?;
	let gradients = output_tensor(4, DType::F32, &[2, 3])?;
	let inputs = [
		NamedTensor::new("scores", &scores),
		NamedTensor::new("labels", &labels),
	];
	let outputs = [
		NamedTensor::new("losses", &losses),
		NamedTensor::new("gradients", &gradients),
	];
	let hinge = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_hinge_loss")?,
		&inputs,
		&outputs,
		"scores",
		&PreparedParameters::new(),
		namespace(30_000),
		ByteCount::new(0),
	))?;
	assert_fragment(&hinge, 0, 1, None)?;
	assert!(opcodes(&hinge).contains(&ScalarOpcode::Select));

	let log_probabilities = input_tensor(11, DType::F32, &[2, 3])?;
	let targets = input_tensor(12, DType::F32, &[2, 3])?;
	let losses = output_tensor(13, DType::F32, &[2, 3])?;
	let inputs = [
		NamedTensor::new("log_probabilities", &log_probabilities),
		NamedTensor::new("targets", &targets),
	];
	let outputs = [NamedTensor::new("losses", &losses)];
	let kl = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_kl_div_loss")?,
		&inputs,
		&outputs,
		"log_probabilities",
		&PreparedParameters::new(),
		namespace(32_000),
		ByteCount::new(48),
	))?;
	assert_fragment(&kl, 48, 1, None)?;
	assert_eq!(kl.stages()[0].kernels().len(), 3);
	assert!(opcodes(&kl).contains(&ScalarOpcode::Require));

	let values = input_tensor(21, DType::F32, &[2, 3])?;
	let mean = output_tensor(22, DType::F32, &[1])?;
	let inputs = [NamedTensor::new("values", &values)];
	let outputs = [NamedTensor::new("mean", &mean)];
	let prepared = tree_parameters(32);
	let global_mean = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_mean_all")?,
		&inputs,
		&outputs,
		"values",
		&prepared,
		namespace(34_000),
		ByteCount::new(4),
	))?;
	assert_fragment(&global_mean, 4, 2, Some(32))?;
	assert_eq!(reductions(&global_mean)[0].axes.as_slice(), [0, 1]);

	for (index, (symbol, output_name, workspace, stages)) in [
		("gpu_mse_into", "mean_squared_error", 28, 3),
		("gpu_ss_res_into", "sum_squared_residuals", 24, 2),
	]
	.into_iter()
	.enumerate()
	{
		let predictions = input_tensor(31, DType::F32, &[2, 3])?;
		let targets = input_tensor(32, DType::F32, &[2, 3])?;
		let result = output_tensor(33, DType::F32, &[1])?;
		let loss_inputs = [
			NamedTensor::new("predictions", &predictions),
			NamedTensor::new("targets", &targets),
		];
		let loss_outputs = [NamedTensor::new(output_name, &result)];
		let materialized = materialize_composition(MaterializationRequest::new(
			operation_registry().resolve_unique(symbol)?,
			&loss_inputs,
			&loss_outputs,
			"predictions",
			&prepared,
			namespace(36_000 + u64::try_from(index)? * 2_000),
			ByteCount::new(workspace),
		))?;
		assert_fragment(&materialized, workspace, stages, Some(32))?;
	}
	Ok(())
}

#[test]
fn materializes_column_statistics_without_hidden_scratch() -> TestResult {
	let values = input_tensor(1, DType::F32, &[3, 4])?;
	let means = output_tensor(2, DType::F32, &[4])?;
	let inputs = [NamedTensor::new("values", &values)];
	let outputs = [NamedTensor::new("means", &means)];
	let prepared = tree_parameters(64);
	let mean = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_reduce_mean_cols")?,
		&inputs,
		&outputs,
		"values",
		&prepared,
		namespace(40_000),
		ByteCount::new(16),
	))?;
	assert_fragment(&mean, 16, 2, Some(64))?;
	assert_eq!(reductions(&mean)[0].axes.as_slice(), [0]);

	let variances = output_tensor(3, DType::F32, &[4])?;
	let outputs = [NamedTensor::new("variances", &variances)];
	let variance = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_reduce_var_cols")?,
		&inputs,
		&outputs,
		"values",
		&prepared,
		namespace(42_000),
		ByteCount::new(96),
	))?;
	assert_fragment(&variance, 96, 3, Some(64))?;
	assert_eq!(variance.stages()[2].kernels().len(), 3);
	assert_eq!(
		variance
			.workspace()
			.objects()
			.iter()
			.map(|object| object.shape().extents())
			.collect::<Vec<_>>(),
		[&[4][..], &[3, 4], &[1, 4], &[1, 4]]
	);
	Ok(())
}

#[test]
fn materializes_distance_losses_at_per_example_granularity() -> TestResult {
	let left = input_tensor(1, DType::F32, &[2, 3])?;
	let right = input_tensor(2, DType::F32, &[2, 3])?;
	let labels = input_tensor(3, DType::F32, &[2])?;
	let losses = output_tensor(4, DType::F32, &[2])?;
	let inputs = [
		NamedTensor::new("left", &left),
		NamedTensor::new("right", &right),
		NamedTensor::new("labels", &labels),
	];
	let outputs = [NamedTensor::new("losses", &losses)];
	let prepared = margin_parameters(0.5, 64);
	let contrastive = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_contrastive_loss")?,
		&inputs,
		&outputs,
		"left",
		&prepared,
		namespace(50_000),
		ByteCount::new(32),
	))?;
	assert_fragment(&contrastive, 32, 3, Some(64))?;
	assert!(opcodes(&contrastive).contains(&ScalarOpcode::SquareRoot));
	assert!(opcodes(&contrastive).contains(&ScalarOpcode::Require));

	let cosine = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_cosine_embedding_loss")?,
		&inputs,
		&outputs,
		"left",
		&prepared,
		namespace(52_000),
		ByteCount::new(96),
	))?;
	assert_fragment(&cosine, 96, 5, Some(64))?;
	assert_eq!(reductions(&cosine).len(), 3);
	assert!(
		reductions(&cosine)
			.iter()
			.all(|reduction| reduction.axes.as_slice() == [1])
	);

	let anchor = input_tensor(11, DType::F32, &[2, 3])?;
	let positive = input_tensor(12, DType::F32, &[2, 3])?;
	let negative = input_tensor(13, DType::F32, &[2, 3])?;
	let losses = output_tensor(14, DType::F32, &[2])?;
	let inputs = [
		NamedTensor::new("anchor", &anchor),
		NamedTensor::new("positive", &positive),
		NamedTensor::new("negative", &negative),
	];
	let outputs = [NamedTensor::new("losses", &losses)];
	let triplet = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_triplet_loss")?,
		&inputs,
		&outputs,
		"anchor",
		&prepared,
		namespace(54_000),
		ByteCount::new(64),
	))?;
	assert_fragment(&triplet, 64, 4, Some(64))?;
	assert_eq!(reductions(&triplet).len(), 2);
	Ok(())
}

#[test]
fn rejects_abi_domain_tree_and_workspace_drift() -> TestResult {
	let predictions = input_tensor(1, DType::F32, &[6])?;
	let targets = input_tensor(2, DType::F32, &[6])?;
	let accuracy = output_tensor(3, DType::F32, &[1])?;
	let inputs = [
		NamedTensor::new("predictions", &predictions),
		NamedTensor::new("targets", &targets),
	];
	let outputs = [NamedTensor::new("accuracy", &accuracy)];
	let bad_tree = tree_parameters(3);
	let error = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_accuracy_into")?,
		&inputs,
		&outputs,
		"predictions",
		&bad_tree,
		namespace(60_000),
		ByteCount::new(28),
	))
	.expect_err("a non-power-of-two reduction tree was accepted");
	assert_eq!(
		error.kind,
		OperationErrorKind::InvalidMaterializationRequest
	);

	let mut extra_parameter = tree_parameters(64);
	extra_parameter.insert(
		"threshold".to_owned(),
		PreparedParameter::F32Bits(0.5f32.to_bits()),
	);
	let error = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_accuracy_into")?,
		&inputs,
		&outputs,
		"predictions",
		&extra_parameter,
		namespace(62_000),
		ByteCount::new(28),
	))
	.expect_err("an undeclared metric parameter was accepted");
	assert_eq!(
		error.kind,
		OperationErrorKind::InvalidMaterializationRequest
	);

	let prepared = tree_parameters(64);
	let error = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_accuracy_into")?,
		&inputs,
		&outputs,
		"predictions",
		&prepared,
		namespace(64_000),
		ByteCount::new(27),
	))
	.expect_err("one byte less than exact workspace was accepted");
	assert_eq!(error.kind, OperationErrorKind::WorkspaceLimitExceeded);

	let oversized = input_tensor(11, DType::F32, &[16_777_217])?;
	let oversized_mean = output_tensor(12, DType::F32, &[1])?;
	let inputs = [NamedTensor::new("values", &oversized)];
	let outputs = [NamedTensor::new("mean", &oversized_mean)];
	let error = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_mean_all")?,
		&inputs,
		&outputs,
		"values",
		&prepared,
		namespace(66_000),
		ByteCount::new(4),
	))
	.expect_err("an element count not losslessly representable in f32 was accepted");
	assert_eq!(error.kind, OperationErrorKind::UnsupportedConcreteShape);

	let left = input_tensor(21, DType::F32, &[2, 3])?;
	let right = input_tensor(22, DType::F32, &[2, 3])?;
	let labels = input_tensor(23, DType::F32, &[2])?;
	let losses = output_tensor(24, DType::F32, &[2])?;
	let inputs = [
		NamedTensor::new("left", &left),
		NamedTensor::new("right", &right),
		NamedTensor::new("labels", &labels),
	];
	let outputs = [NamedTensor::new("losses", &losses)];
	let nonfinite_margin = margin_parameters(f32::INFINITY, 64);
	let error = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_contrastive_loss")?,
		&inputs,
		&outputs,
		"left",
		&nonfinite_margin,
		namespace(68_000),
		ByteCount::new(32),
	))
	.expect_err("a nonfinite distance margin was accepted");
	assert_eq!(
		error.kind,
		OperationErrorKind::InvalidMaterializationRequest
	);
	Ok(())
}
