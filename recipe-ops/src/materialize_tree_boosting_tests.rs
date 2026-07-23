use std::error::Error;

use recipe_core::{ByteCount, DType, KernelTemplateId, ValueId};
use recipe_language::{Shape, Tensor};

use crate::{
	IdentityNamespace, MaterializationRequest, MaterializedComposition, NamedTensor, PreparedParameter,
	PreparedParameters, materialize_composition, operation_registry, remaining_composition_manifest,
};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

const MATERIALIZED: &[(&str, &str)] = &[
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

fn materialize(
	symbol: &str,
	source: &str,
	inputs: &[NamedTensor<'_>],
	outputs: &[NamedTensor<'_>],
	iteration_input: &str,
	prepared: &PreparedParameters,
	seed: u64,
) -> TestResult<MaterializedComposition> {
	let descriptor = operation_registry().resolve_exact(symbol, source)?;
	let materialized = materialize_composition(MaterializationRequest::new(
		descriptor,
		inputs,
		outputs,
		iteration_input,
		prepared,
		namespace(seed),
		ByteCount::new(1_000_000),
	))?;
	materialized.graph().validate()?;
	assert_eq!(
		materialized.stages().len(),
		materialized.resolved().steps().len()
	);
	Ok(materialized)
}

#[test]
fn source_qualified_tree_batch_leaves_only_honest_recipes_materialized() -> TestResult {
	let remaining = remaining_composition_manifest();
	for (symbol, source) in MATERIALIZED {
		assert!(
			remaining
				.iter()
				.all(|entry| entry.symbol() != *symbol || entry.source() != *source),
			"({symbol}, {source}) remained descriptive"
		);
	}
	for (symbol, source) in [
		("gpu_oob_mask", "gpu-core/src/forest.rs:174"),
		("gpu_tb_scatter", "gpu-core/src/kernels.rs:3881"),
	] {
		assert!(
			remaining
				.iter()
				.any(|entry| entry.symbol() == symbol && entry.source() == source),
			"({symbol}, {source}) should fail closed until its recipe order is corrected"
		);
	}
	Ok(())
}

#[test]
fn materializes_split_argmax_bootstrap_feature_subset_and_gradients() -> TestResult {
	let gains = input_tensor(1, DType::F32, &[2, 3])?;
	let best_gain = output_tensor(2, DType::F32, &[1])?;
	let best_feature = output_tensor(3, DType::I32, &[1])?;
	let best_bin = output_tensor(4, DType::I32, &[1])?;
	let inputs = [NamedTensor::new("gains", &gains)];
	let outputs = [
		NamedTensor::new("best_gain", &best_gain),
		NamedTensor::new("best_feature", &best_feature),
		NamedTensor::new("best_bin", &best_bin),
	];
	let prepared = parameters(&[
		("features", PreparedParameter::U64(2)),
		("bins", PreparedParameter::U64(3)),
		("tree_lanes", PreparedParameter::U64(4)),
	]);
	let result = materialize(
		"gpu_argmax_write_split",
		"gpu-core/src/kernels.rs:4155",
		&inputs,
		&outputs,
		"gains",
		&prepared,
		10_000,
	)?;
	assert_eq!(result.graph().nodes.len(), 2);

	let row_ids = input_tensor(11, DType::I32, &[4])?;
	let sampled_rows = output_tensor(12, DType::I32, &[6])?;
	let inputs = [NamedTensor::new("row_ids", &row_ids)];
	let outputs = [NamedTensor::new("sampled_rows", &sampled_rows)];
	let prepared = parameters(&[
		("rows", PreparedParameter::U64(4)),
		("samples", PreparedParameter::U64(6)),
		("seed_low", PreparedParameter::U64(11)),
		("seed_high", PreparedParameter::U64(17)),
		("stream", PreparedParameter::U64(23)),
	]);
	let result = materialize(
		"gpu_bootstrap_sample",
		"gpu-core/src/forest.rs:58",
		&inputs,
		&outputs,
		"row_ids",
		&prepared,
		20_000,
	)?;
	assert_eq!(result.graph().nodes.len(), 2);

	let feature_ids = input_tensor(21, DType::I32, &[5])?;
	let selection_indices = input_tensor(22, DType::I32, &[3])?;
	let selected_features = output_tensor(23, DType::I32, &[3])?;
	let inputs = [
		NamedTensor::new("feature_ids", &feature_ids),
		NamedTensor::new("selection_indices", &selection_indices),
	];
	let outputs = [NamedTensor::new("selected_features", &selected_features)];
	let prepared = parameters(&[
		("features", PreparedParameter::U64(5)),
		("selected", PreparedParameter::U64(3)),
		("selection_indices_verified", PreparedParameter::Bool(true)),
		("selection_indices_unique", PreparedParameter::Bool(true)),
	]);
	let result = materialize(
		"gpu_feature_subset",
		"gpu-core/src/forest.rs:82",
		&inputs,
		&outputs,
		"feature_ids",
		&prepared,
		30_000,
	)?;
	assert_eq!(result.graph().nodes.len(), 2);

	let probabilities = input_tensor(31, DType::F32, &[4])?;
	let targets = input_tensor(32, DType::I32, &[4])?;
	let weights = input_tensor(33, DType::F32, &[4])?;
	let mask = input_tensor(34, DType::I32, &[4])?;
	let gradients = output_tensor(35, DType::F32, &[4])?;
	let hessians = output_tensor(36, DType::F32, &[4])?;
	let inputs = [
		NamedTensor::new("probabilities", &probabilities),
		NamedTensor::new("targets", &targets),
		NamedTensor::new("weights", &weights),
		NamedTensor::new("mask", &mask),
	];
	let outputs = [
		NamedTensor::new("gradients", &gradients),
		NamedTensor::new("hessians", &hessians),
	];
	let prepared = parameters(&[
		("rows", PreparedParameter::U64(4)),
		("class_index", PreparedParameter::U64(2)),
	]);
	let result = materialize(
		"gpu_grad_hess_into",
		"gpu-core/src/kernels.rs:3703",
		&inputs,
		&outputs,
		"probabilities",
		&prepared,
		40_000,
	)?;
	assert_eq!(result.graph().nodes.len(), 1);
	Ok(())
}

#[test]
fn materializes_all_histogram_aliases() -> TestResult {
	for (index, (symbol, source, include_counts)) in [
		("gpu_histogram_build", "gpu-core/src/kernels.rs:6745", true),
		("gpu_lgbm_histogram", "gpu-core/src/kernels.rs:7018", true),
		(
			"gpu_oblivious_histogram",
			"gpu-core/src/kernels.rs:4205",
			false,
		),
		("gpu_tb_histogram", "gpu-core/src/kernels.rs:3737", false),
	]
	.into_iter()
	.enumerate()
	{
		let bins = input_tensor(1, DType::I32, &[6])?;
		let gradients = input_tensor(2, DType::F32, &[6])?;
		let hessians = input_tensor(3, DType::F32, &[6])?;
		let gradient_histogram = output_tensor(4, DType::F32, &[4])?;
		let hessian_histogram = output_tensor(5, DType::F32, &[4])?;
		let count_histogram = output_tensor(6, DType::I32, &[4])?;
		let inputs = [
			NamedTensor::new("bin_indices", &bins),
			NamedTensor::new("gradient_contributions", &gradients),
			NamedTensor::new("hessian_contributions", &hessians),
		];
		let outputs = match include_counts {
			true => vec![
				NamedTensor::new("gradient_histogram", &gradient_histogram),
				NamedTensor::new("hessian_histogram", &hessian_histogram),
				NamedTensor::new("count_histogram", &count_histogram),
			],
			false => vec![
				NamedTensor::new("gradient_histogram", &gradient_histogram),
				NamedTensor::new("hessian_histogram", &hessian_histogram),
			],
		};
		let prepared = parameters(&[
			("contributions", PreparedParameter::U64(6)),
			("histogram_bins", PreparedParameter::U64(4)),
			("contribution_table_verified", PreparedParameter::Bool(true)),
		]);
		let result = materialize(
			symbol,
			source,
			&inputs,
			&outputs,
			"bin_indices",
			&prepared,
			50_000 + u64::try_from(index)? * 2_000,
		)?;
		assert_eq!(result.graph().nodes.len(), 3 + usize::from(include_counts));
	}
	Ok(())
}

#[test]
fn materializes_regularized_leaves_reductions_and_histogram_subtraction() -> TestResult {
	for (index, (symbol, source, expose_sums)) in [
		("gpu_leaf_finalize", "gpu-core/src/kernels.rs:4372", false),
		("gpu_tb_leaf_val", "gpu-core/src/kernels.rs:3856", false),
		("gpu_lgbm_leaf_reduce", "gpu-core/src/kernels.rs:7119", true),
		("gpu_tb_leaf_sum", "gpu-core/src/kernels.rs:3830", true),
	]
	.into_iter()
	.enumerate()
	{
		let leaf_indices = input_tensor(1, DType::I32, &[4])?;
		let gradients = input_tensor(2, DType::F32, &[4])?;
		let hessians = input_tensor(3, DType::F32, &[4])?;
		let leaf_gradient = output_tensor(4, DType::F32, &[1])?;
		let leaf_hessian = output_tensor(5, DType::F32, &[1])?;
		let leaf_value = output_tensor(6, DType::F32, &[1])?;
		let inputs = [
			NamedTensor::new("leaf_indices", &leaf_indices),
			NamedTensor::new("gradient_contributions", &gradients),
			NamedTensor::new("hessian_contributions", &hessians),
		];
		let outputs = match expose_sums {
			true => vec![
				NamedTensor::new("leaf_gradient", &leaf_gradient),
				NamedTensor::new("leaf_hessian", &leaf_hessian),
				NamedTensor::new("leaf_value", &leaf_value),
			],
			false => vec![NamedTensor::new("leaf_value", &leaf_value)],
		};
		let prepared = parameters(&[
			("contributions", PreparedParameter::U64(4)),
			("leaves", PreparedParameter::U64(1)),
			("lambda", PreparedParameter::F32Bits(1.0_f32.to_bits())),
			("contribution_table_verified", PreparedParameter::Bool(true)),
			("tree_lanes", PreparedParameter::U64(1)),
		]);
		let result = materialize(
			symbol,
			source,
			&inputs,
			&outputs,
			"leaf_indices",
			&prepared,
			70_000 + u64::try_from(index)? * 2_000,
		)?;
		assert_eq!(result.graph().nodes.len(), 5);
	}

	let leaf_indices = input_tensor(11, DType::I32, &[4])?;
	let gradients = input_tensor(12, DType::F32, &[4])?;
	let hessians = input_tensor(13, DType::F32, &[4])?;
	let gradient_base = input_tensor(14, DType::F32, &[2])?;
	let hessian_base = input_tensor(15, DType::F32, &[2])?;
	let leaf_gradients = output_tensor(16, DType::F32, &[2])?;
	let leaf_hessians = output_tensor(17, DType::F32, &[2])?;
	let inputs = [
		NamedTensor::new("leaf_indices", &leaf_indices),
		NamedTensor::new("gradients", &gradients),
		NamedTensor::new("hessians", &hessians),
		NamedTensor::new("gradient_base", &gradient_base),
		NamedTensor::new("hessian_base", &hessian_base),
	];
	let outputs = [
		NamedTensor::new("leaf_gradients", &leaf_gradients),
		NamedTensor::new("leaf_hessians", &leaf_hessians),
	];
	let prepared = parameters(&[
		("rows", PreparedParameter::U64(4)),
		("leaves", PreparedParameter::U64(2)),
		("leaf_indices_verified", PreparedParameter::Bool(true)),
		("bases_zero", PreparedParameter::Bool(true)),
	]);
	let result = materialize(
		"gpu_leaf_reduce",
		"gpu-core/src/kernels.rs:4346",
		&inputs,
		&outputs,
		"leaf_indices",
		&prepared,
		90_000,
	)?;
	assert_eq!(result.graph().nodes.len(), 3);

	let parent_gradients = input_tensor(21, DType::F32, &[4])?;
	let parent_hessians = input_tensor(22, DType::F32, &[4])?;
	let parent_counts = input_tensor(23, DType::I32, &[4])?;
	let child_gradients = input_tensor(24, DType::F32, &[4])?;
	let child_hessians = input_tensor(25, DType::F32, &[4])?;
	let child_counts = input_tensor(26, DType::I32, &[4])?;
	let remaining_gradients = output_tensor(27, DType::F32, &[4])?;
	let remaining_hessians = output_tensor(28, DType::F32, &[4])?;
	let remaining_counts = output_tensor(29, DType::I32, &[4])?;
	let inputs = [
		NamedTensor::new("parent_gradients", &parent_gradients),
		NamedTensor::new("parent_hessians", &parent_hessians),
		NamedTensor::new("parent_counts", &parent_counts),
		NamedTensor::new("child_gradients", &child_gradients),
		NamedTensor::new("child_hessians", &child_hessians),
		NamedTensor::new("child_counts", &child_counts),
	];
	let outputs = [
		NamedTensor::new("remaining_gradients", &remaining_gradients),
		NamedTensor::new("remaining_hessians", &remaining_hessians),
		NamedTensor::new("remaining_counts", &remaining_counts),
	];
	let prepared = parameters(&[("elements", PreparedParameter::U64(4))]);
	let result = materialize(
		"gpu_lgbm_hist_subtract",
		"gpu-core/src/kernels.rs:7053",
		&inputs,
		&outputs,
		"parent_gradients",
		&prepared,
		92_000,
	)?;
	assert_eq!(result.graph().nodes.len(), 1);
	Ok(())
}

#[test]
fn materializes_the_exact_one_feature_two_bin_split_domain() -> TestResult {
	for (index, (symbol, source)) in [
		("gpu_lgbm_best_split", "gpu-core/src/kernels.rs:7080"),
		("gpu_oblivious_split_eval", "gpu-core/src/kernels.rs:4396"),
		("gpu_split_eval", "gpu-core/src/kernels.rs:6778"),
		("gpu_tb_split_eval", "gpu-core/src/kernels.rs:3770"),
	]
	.into_iter()
	.enumerate()
	{
		let bin_indices = input_tensor(1, DType::I32, &[2])?;
		let gradient_histogram = input_tensor(2, DType::F32, &[2])?;
		let hessian_histogram = input_tensor(3, DType::F32, &[2])?;
		let count_histogram = input_tensor(4, DType::I32, &[2])?;
		let left_count_index = input_tensor(5, DType::I32, &[1])?;
		let total_index = input_tensor(6, DType::I32, &[1])?;
		let feature_zero = input_tensor(7, DType::I32, &[1])?;
		let best_gain = output_tensor(8, DType::F32, &[1])?;
		let best_feature = output_tensor(9, DType::I32, &[1])?;
		let best_bin = output_tensor(10, DType::I32, &[1])?;
		let best_left_count = output_tensor(11, DType::I32, &[1])?;
		let inputs = [
			NamedTensor::new("bin_indices", &bin_indices),
			NamedTensor::new("gradient_histogram", &gradient_histogram),
			NamedTensor::new("hessian_histogram", &hessian_histogram),
			NamedTensor::new("count_histogram", &count_histogram),
			NamedTensor::new("left_count_index", &left_count_index),
			NamedTensor::new("total_index", &total_index),
			NamedTensor::new("feature_zero", &feature_zero),
		];
		let outputs = [
			NamedTensor::new("best_gain", &best_gain),
			NamedTensor::new("best_feature", &best_feature),
			NamedTensor::new("best_bin", &best_bin),
			NamedTensor::new("best_left_count", &best_left_count),
		];
		let prepared = parameters(&[
			("features", PreparedParameter::U64(1)),
			("bins", PreparedParameter::U64(2)),
			("nodes", PreparedParameter::U64(1)),
			("lambda", PreparedParameter::F32Bits(1.0_f32.to_bits())),
			(
				"minimum_child_weight",
				PreparedParameter::F32Bits(0.0_f32.to_bits()),
			),
			("bin_indices_identity", PreparedParameter::Bool(true)),
			("left_count_index_zero", PreparedParameter::Bool(true)),
			("total_index_one", PreparedParameter::Bool(true)),
			("feature_zero_is_zero", PreparedParameter::Bool(true)),
			("count_histogram_nonnegative", PreparedParameter::Bool(true)),
			("positive_gain_guaranteed", PreparedParameter::Bool(true)),
			("tree_lanes", PreparedParameter::U64(2)),
		]);
		let result = materialize(
			symbol,
			source,
			&inputs,
			&outputs,
			"gradient_histogram",
			&prepared,
			100_000 + u64::try_from(index)? * 2_000,
		)?;
		assert_eq!(result.graph().nodes.len(), 11);
	}
	Ok(())
}

#[test]
fn materializes_catboost_random_threshold_leaf_updates_and_split_writes() -> TestResult {
	let ordering_keys = input_tensor(1, DType::I32, &[4])?;
	let targets = input_tensor(2, DType::F32, &[4])?;
	let sum_base = input_tensor(3, DType::F32, &[4])?;
	let count_base = input_tensor(4, DType::F32, &[4])?;
	let encoded = output_tensor(5, DType::F32, &[4])?;
	let inputs = [
		NamedTensor::new("ordering_keys", &ordering_keys),
		NamedTensor::new("targets", &targets),
		NamedTensor::new("sum_base", &sum_base),
		NamedTensor::new("count_base", &count_base),
	];
	let outputs = [NamedTensor::new("encoded", &encoded)];
	let prepared = parameters(&[
		("rows", PreparedParameter::U64(4)),
		("prior", PreparedParameter::F32Bits(0.5_f32.to_bits())),
		("smoothing", PreparedParameter::F32Bits(1.0_f32.to_bits())),
		("one_category", PreparedParameter::Bool(true)),
		("ordering_keys_unique", PreparedParameter::Bool(true)),
		("bases_zero", PreparedParameter::Bool(true)),
		("tree_lanes", PreparedParameter::U64(4)),
	]);
	let result = materialize(
		"gpu_ordered_target_stats",
		"gpu-core/src/catboost.rs:100",
		&inputs,
		&outputs,
		"ordering_keys",
		&prepared,
		120_000,
	)?;
	assert_eq!(result.graph().nodes.len(), 8);

	let minimum = input_tensor(11, DType::F32, &[1])?;
	let maximum = input_tensor(12, DType::F32, &[1])?;
	let threshold = output_tensor(13, DType::F32, &[1])?;
	let inputs = [
		NamedTensor::new("minimum", &minimum),
		NamedTensor::new("maximum", &maximum),
	];
	let outputs = [NamedTensor::new("threshold", &threshold)];
	let prepared = parameters(&[
		("seed_low", PreparedParameter::U64(31)),
		("seed_high", PreparedParameter::U64(37)),
		("stream", PreparedParameter::U64(41)),
	]);
	let result = materialize(
		"gpu_random_threshold_split",
		"gpu-core/src/forest.rs:108",
		&inputs,
		&outputs,
		"minimum",
		&prepared,
		122_000,
	)?;
	assert_eq!(result.graph().nodes.len(), 2);

	for (index, (symbol, source)) in [
		("gpu_scatter_add_by_leaf", "gpu-core/src/kernels.rs:4323"),
		(
			"gpu_scatter_add_by_leaf_col",
			"gpu-core/src/kernels.rs:4497",
		),
	]
	.into_iter()
	.enumerate()
	{
		let prediction_base = input_tensor(21, DType::F32, &[5])?;
		let leaf_values = input_tensor(22, DType::F32, &[2])?;
		let leaf_indices = input_tensor(23, DType::I32, &[3])?;
		let destination_indices = input_tensor(24, DType::I32, &[3])?;
		let predictions = output_tensor(25, DType::F32, &[5])?;
		let inputs = [
			NamedTensor::new("prediction_base", &prediction_base),
			NamedTensor::new("leaf_values", &leaf_values),
			NamedTensor::new("leaf_indices", &leaf_indices),
			NamedTensor::new("destination_indices", &destination_indices),
		];
		let outputs = [NamedTensor::new("predictions", &predictions)];
		let prepared = parameters(&[
			("prediction_elements", PreparedParameter::U64(5)),
			("updates", PreparedParameter::U64(3)),
			("leaves", PreparedParameter::U64(2)),
			(
				"learning_rate",
				PreparedParameter::F32Bits(0.25_f32.to_bits()),
			),
			("leaf_indices_verified", PreparedParameter::Bool(true)),
			("destination_indices_unique", PreparedParameter::Bool(true)),
		]);
		let result = materialize(
			symbol,
			source,
			&inputs,
			&outputs,
			"prediction_base",
			&prepared,
			124_000 + u64::try_from(index)? * 2_000,
		)?;
		assert_eq!(result.graph().nodes.len(), 4);
	}

	let feature_base = input_tensor(31, DType::I32, &[4])?;
	let bin_base = input_tensor(32, DType::I32, &[4])?;
	let write_mask = input_tensor(33, DType::I32, &[4])?;
	let split_features = output_tensor(34, DType::I32, &[4])?;
	let split_bins = output_tensor(35, DType::I32, &[4])?;
	let inputs = [
		NamedTensor::new("feature_base", &feature_base),
		NamedTensor::new("bin_base", &bin_base),
		NamedTensor::new("write_mask", &write_mask),
	];
	let outputs = [
		NamedTensor::new("split_features", &split_features),
		NamedTensor::new("split_bins", &split_bins),
	];
	let prepared = parameters(&[
		("slots", PreparedParameter::U64(4)),
		("feature", PreparedParameter::U64(3)),
		("bin", PreparedParameter::U64(2)),
		("write_mask_one_hot", PreparedParameter::Bool(true)),
	]);
	let result = materialize(
		"gpu_write_split",
		"gpu-core/src/kernels.rs:4182",
		&inputs,
		&outputs,
		"feature_base",
		&prepared,
		130_000,
	)?;
	assert_eq!(result.graph().nodes.len(), 1);
	Ok(())
}

#[test]
fn preserves_the_existing_exact_leaf_split_route() -> TestResult {
	let features = input_tensor(1, DType::F32, &[6])?;
	let feature_indices = input_tensor(2, DType::I32, &[3])?;
	let thresholds = input_tensor(3, DType::F32, &[3])?;
	let assignments = input_tensor(4, DType::I32, &[3])?;
	let destination_indices = input_tensor(5, DType::I32, &[3])?;
	let updated_assignments = output_tensor(6, DType::I32, &[3])?;
	let inputs = [
		NamedTensor::new("features", &features),
		NamedTensor::new("feature_indices", &feature_indices),
		NamedTensor::new("thresholds", &thresholds),
		NamedTensor::new("assignments", &assignments),
		NamedTensor::new("destination_indices", &destination_indices),
	];
	let outputs = [NamedTensor::new(
		"updated_assignments",
		&updated_assignments,
	)];
	let prepared = parameters(&[("destination_indices_unique", PreparedParameter::Bool(true))]);
	let result = materialize(
		"gpu_leaf_split_apply",
		"gpu-core/src/kernels.rs:7173",
		&inputs,
		&outputs,
		"features",
		&prepared,
		140_000,
	)?;
	assert_eq!(result.graph().nodes.len(), 3);
	Ok(())
}
