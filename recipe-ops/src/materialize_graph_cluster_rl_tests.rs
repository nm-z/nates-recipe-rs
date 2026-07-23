use std::error::Error;

use recipe_core::{ByteCount, DType, KernelTemplateId, ValueId};
use recipe_language::{Shape, Tensor};

use crate::{
	IdentityNamespace, MaterializationRequest, MaterializedComposition, NamedTensor, OperationErrorKind,
	PreparedParameter, PreparedParameters, materialize_composition, operation_registry,
	remaining_composition_manifest,
};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

const MATERIALIZED: &[&str] = &[
	"gpu_boruvka_mst",
	"gpu_categorical_logprob",
	"gpu_centroid_update",
	"gpu_core_distance",
	"gpu_csr_spmm",
	"gpu_csr_spmv",
	"gpu_degree",
	"gpu_discounted_returns",
	"gpu_fixed_radius_neighbors",
	"gpu_gae",
	"gpu_gaussian_logprob",
	"gpu_gcn_norm",
	"gpu_neighbor_aggregate",
	"gpu_td_targets",
	"gpu_union_find_cc",
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

fn assert_fragment(materialized: &MaterializedComposition, workspace: u64, stages: usize, nodes: usize) -> TestResult {
	materialized.graph().validate()?;
	assert_eq!(materialized.workspace().bytes(), ByteCount::new(workspace));
	assert_eq!(materialized.stages().len(), stages);
	assert_eq!(materialized.graph().nodes.len(), nodes);
	Ok(())
}

#[test]
fn source_qualified_batch_is_removed_from_the_remaining_manifest() -> TestResult {
	let remaining = remaining_composition_manifest();
	for symbol in MATERIALIZED {
		assert!(
			remaining.iter().all(|entry| entry.symbol() != *symbol),
			"{symbol} remained descriptive"
		);
	}
	Ok(())
}

#[test]
fn materializes_degree_td_and_gcn_graphs() -> TestResult {
	let destinations = input_tensor(1, DType::I32, &[3])?;
	let degrees = output_tensor(2, DType::I32, &[2])?;
	let inputs = [NamedTensor::new("edge_destinations", &destinations)];
	let outputs = [NamedTensor::new("degrees", &degrees)];
	let prepared = parameters(&[
		("nodes", PreparedParameter::U64(2)),
		("edges", PreparedParameter::U64(3)),
		("endpoint_indices_verified", PreparedParameter::Bool(true)),
	]);
	let degree = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_degree")?,
		&inputs,
		&outputs,
		"edge_destinations",
		&prepared,
		namespace(10_000),
		ByteCount::new(12),
	))?;
	assert_fragment(&degree, 12, 2, 2)?;

	let rewards = input_tensor(11, DType::F32, &[3])?;
	let values = input_tensor(12, DType::F32, &[3])?;
	let next_indices = input_tensor(13, DType::I32, &[3])?;
	let done = input_tensor(14, DType::I32, &[3])?;
	let targets = output_tensor(15, DType::F32, &[3])?;
	let inputs = [
		NamedTensor::new("rewards", &rewards),
		NamedTensor::new("values", &values),
		NamedTensor::new("next_value_indices", &next_indices),
		NamedTensor::new("done_mask", &done),
	];
	let outputs = [NamedTensor::new("targets", &targets)];
	let prepared = parameters(&[
		("elements", PreparedParameter::U64(3)),
		("gamma", PreparedParameter::F32Bits(0.9_f32.to_bits())),
		("next_value_indices_verified", PreparedParameter::Bool(true)),
	]);
	let td = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_td_targets")?,
		&inputs,
		&outputs,
		"rewards",
		&prepared,
		namespace(20_000),
		ByteCount::new(12),
	))?;
	assert_fragment(&td, 12, 2, 2)?;

	let features = input_tensor(21, DType::F32, &[4])?;
	let degree_values = input_tensor(22, DType::F32, &[2])?;
	let degree_indices = input_tensor(23, DType::I32, &[4])?;
	let normalized = output_tensor(24, DType::F32, &[4])?;
	let inputs = [
		NamedTensor::new("features", &features),
		NamedTensor::new("degrees", &degree_values),
		NamedTensor::new("degree_indices", &degree_indices),
	];
	let outputs = [NamedTensor::new("normalized", &normalized)];
	let prepared = parameters(&[
		("nodes", PreparedParameter::U64(2)),
		("features_per_node", PreparedParameter::U64(2)),
		("degree_indices_verified", PreparedParameter::Bool(true)),
	]);
	let gcn = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_gcn_norm")?,
		&inputs,
		&outputs,
		"features",
		&prepared,
		namespace(30_000),
		ByteCount::new(16),
	))?;
	assert_fragment(&gcn, 16, 2, 2)
}

#[test]
fn materializes_reinforcement_learning_scans_and_log_probabilities() -> TestResult {
	let rewards = input_tensor(1, DType::F32, &[4])?;
	let returns = output_tensor(2, DType::F32, &[4])?;
	let inputs = [NamedTensor::new("rewards", &rewards)];
	let outputs = [NamedTensor::new("returns", &returns)];
	let prepared = parameters(&[
		("length", PreparedParameter::U64(4)),
		("gamma", PreparedParameter::F32Bits(1.0_f32.to_bits())),
		("tree_lanes", PreparedParameter::U64(4)),
	]);
	let discounted = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_discounted_returns")?,
		&inputs,
		&outputs,
		"rewards",
		&prepared,
		namespace(40_000),
		ByteCount::new(16),
	))?;
	assert_fragment(&discounted, 16, 2, 2)?;

	let rewards = input_tensor(11, DType::F32, &[4])?;
	let values = input_tensor(12, DType::F32, &[4])?;
	let next_indices = input_tensor(13, DType::I32, &[4])?;
	let has_next = input_tensor(14, DType::I32, &[4])?;
	let advantages = output_tensor(15, DType::F32, &[4])?;
	let inputs = [
		NamedTensor::new("rewards", &rewards),
		NamedTensor::new("values", &values),
		NamedTensor::new("next_value_indices", &next_indices),
		NamedTensor::new("has_next", &has_next),
	];
	let outputs = [NamedTensor::new("advantages", &advantages)];
	let prepared = parameters(&[
		("length", PreparedParameter::U64(4)),
		("gamma", PreparedParameter::F32Bits(1.0_f32.to_bits())),
		("lambda", PreparedParameter::F32Bits(1.0_f32.to_bits())),
		("tree_lanes", PreparedParameter::U64(4)),
		("next_value_indices_verified", PreparedParameter::Bool(true)),
	]);
	let gae = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_gae")?,
		&inputs,
		&outputs,
		"rewards",
		&prepared,
		namespace(50_000),
		ByteCount::new(32),
	))?;
	assert_fragment(&gae, 32, 2, 3)?;

	let means = input_tensor(21, DType::F32, &[2, 3])?;
	let log_stds = input_tensor(22, DType::F32, &[2, 3])?;
	let actions = input_tensor(23, DType::F32, &[2, 3])?;
	let log_probabilities = output_tensor(24, DType::F32, &[2])?;
	let inputs = [
		NamedTensor::new("means", &means),
		NamedTensor::new("log_standard_deviations", &log_stds),
		NamedTensor::new("actions", &actions),
	];
	let outputs = [NamedTensor::new("log_probabilities", &log_probabilities)];
	let prepared = parameters(&[
		("rows", PreparedParameter::U64(2)),
		("dimensions", PreparedParameter::U64(3)),
		("tree_lanes", PreparedParameter::U64(4)),
	]);
	let gaussian = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_gaussian_logprob")?,
		&inputs,
		&outputs,
		"means",
		&prepared,
		namespace(60_000),
		ByteCount::new(48),
	))?;
	assert_fragment(&gaussian, 48, 2, 3)?;

	let logits = input_tensor(31, DType::F32, &[2, 3])?;
	let flat_logits = input_tensor(32, DType::F32, &[6])?;
	let selected_actions = input_tensor(33, DType::I32, &[2])?;
	let row_bases = input_tensor(34, DType::I32, &[2])?;
	let log_probabilities = output_tensor(35, DType::F32, &[2])?;
	let inputs = [
		NamedTensor::new("logits", &logits),
		NamedTensor::new("flat_logits", &flat_logits),
		NamedTensor::new("actions", &selected_actions),
		NamedTensor::new("row_bases", &row_bases),
	];
	let outputs = [NamedTensor::new("log_probabilities", &log_probabilities)];
	let prepared = parameters(&[
		("rows", PreparedParameter::U64(2)),
		("action_count", PreparedParameter::U64(3)),
		("tree_lanes", PreparedParameter::U64(4)),
		("logit_views_identical", PreparedParameter::Bool(true)),
		("row_bases_verified", PreparedParameter::Bool(true)),
	]);
	let categorical = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_categorical_logprob")?,
		&inputs,
		&outputs,
		"logits",
		&prepared,
		namespace(70_000),
		ByteCount::new(96),
	))?;
	assert_fragment(&categorical, 96, 2, 9)
}

#[test]
fn materializes_padded_csr_vector_and_matrix_contractions() -> TestResult {
	let values = input_tensor(1, DType::F32, &[3])?;
	let vector = input_tensor(2, DType::F32, &[3])?;
	let value_indices = input_tensor(3, DType::I32, &[2, 2])?;
	let operand_indices = input_tensor(4, DType::I32, &[2, 2])?;
	let active_mask = input_tensor(5, DType::I32, &[2, 2])?;
	let output_base = input_tensor(6, DType::F32, &[2])?;
	let output_indices = input_tensor(7, DType::I32, &[2])?;
	let product = output_tensor(8, DType::F32, &[2])?;
	let inputs = [
		NamedTensor::new("values", &values),
		NamedTensor::new("vector", &vector),
		NamedTensor::new("value_indices", &value_indices),
		NamedTensor::new("operand_indices", &operand_indices),
		NamedTensor::new("active_mask", &active_mask),
		NamedTensor::new("output_base", &output_base),
		NamedTensor::new("output_indices", &output_indices),
	];
	let outputs = [NamedTensor::new("product", &product)];
	let prepared = parameters(&[
		("rows", PreparedParameter::U64(2)),
		("columns", PreparedParameter::U64(3)),
		("nonzeros", PreparedParameter::U64(3)),
		("row_width", PreparedParameter::U64(2)),
		("tables_verified", PreparedParameter::Bool(true)),
		("output_base_zero", PreparedParameter::Bool(true)),
		("output_indices_unique", PreparedParameter::Bool(true)),
		("tree_lanes", PreparedParameter::U64(2)),
	]);
	let spmv = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_csr_spmv")?,
		&inputs,
		&outputs,
		"values",
		&prepared,
		namespace(80_000),
		ByteCount::new(56),
	))?;
	assert_fragment(&spmv, 56, 3, 5)?;

	let values = input_tensor(11, DType::F32, &[3])?;
	let matrix = input_tensor(12, DType::F32, &[6])?;
	let value_indices = input_tensor(13, DType::I32, &[4, 2])?;
	let operand_indices = input_tensor(14, DType::I32, &[4, 2])?;
	let active_mask = input_tensor(15, DType::I32, &[4, 2])?;
	let output_base = input_tensor(16, DType::F32, &[4])?;
	let output_indices = input_tensor(17, DType::I32, &[4])?;
	let product = output_tensor(18, DType::F32, &[4])?;
	let inputs = [
		NamedTensor::new("values", &values),
		NamedTensor::new("matrix", &matrix),
		NamedTensor::new("value_indices", &value_indices),
		NamedTensor::new("operand_indices", &operand_indices),
		NamedTensor::new("active_mask", &active_mask),
		NamedTensor::new("output_base", &output_base),
		NamedTensor::new("output_indices", &output_indices),
	];
	let outputs = [NamedTensor::new("product", &product)];
	let prepared = parameters(&[
		("rows", PreparedParameter::U64(2)),
		("columns", PreparedParameter::U64(3)),
		("features", PreparedParameter::U64(2)),
		("nonzeros", PreparedParameter::U64(3)),
		("row_width", PreparedParameter::U64(2)),
		("tables_verified", PreparedParameter::Bool(true)),
		("output_base_zero", PreparedParameter::Bool(true)),
		("output_indices_unique", PreparedParameter::Bool(true)),
		("tree_lanes", PreparedParameter::U64(2)),
	]);
	let spmm = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_csr_spmm")?,
		&inputs,
		&outputs,
		"values",
		&prepared,
		namespace(90_000),
		ByteCount::new(112),
	))?;
	assert_fragment(&spmm, 112, 3, 5)
}

#[test]
fn materializes_neighbor_and_centroid_reductions_with_fixed_tables() -> TestResult {
	let features = input_tensor(1, DType::F32, &[4])?;
	let feature_indices = input_tensor(2, DType::I32, &[4, 2])?;
	let active_mask = input_tensor(3, DType::I32, &[4, 2])?;
	let degrees = input_tensor(4, DType::F32, &[2])?;
	let degree_indices = input_tensor(5, DType::I32, &[4])?;
	let output_base = input_tensor(6, DType::F32, &[4])?;
	let output_indices = input_tensor(7, DType::I32, &[4])?;
	let aggregate = output_tensor(8, DType::F32, &[4])?;
	let inputs = [
		NamedTensor::new("features", &features),
		NamedTensor::new("feature_indices", &feature_indices),
		NamedTensor::new("active_mask", &active_mask),
		NamedTensor::new("degrees", &degrees),
		NamedTensor::new("degree_indices", &degree_indices),
		NamedTensor::new("output_base", &output_base),
		NamedTensor::new("output_indices", &output_indices),
	];
	let outputs = [NamedTensor::new("aggregate", &aggregate)];
	let prepared = parameters(&[
		("nodes", PreparedParameter::U64(2)),
		("features_per_node", PreparedParameter::U64(2)),
		("row_width", PreparedParameter::U64(2)),
		("mean", PreparedParameter::Bool(true)),
		("tables_verified", PreparedParameter::Bool(true)),
		("output_base_zero", PreparedParameter::Bool(true)),
		("output_indices_unique", PreparedParameter::Bool(true)),
		("tree_lanes", PreparedParameter::U64(2)),
	]);
	let neighbor = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_neighbor_aggregate")?,
		&inputs,
		&outputs,
		"features",
		&prepared,
		namespace(100_000),
		ByteCount::new(112),
	))?;
	assert_fragment(&neighbor, 112, 3, 6)?;

	let points = input_tensor(11, DType::F32, &[6])?;
	let point_indices = input_tensor(12, DType::I32, &[4, 2])?;
	let active_mask = input_tensor(13, DType::I32, &[4, 2])?;
	let assignments = input_tensor(14, DType::I32, &[3])?;
	let count_indices = input_tensor(15, DType::I32, &[4])?;
	let centroid_base = input_tensor(16, DType::F32, &[4])?;
	let centroid_indices = input_tensor(17, DType::I32, &[4])?;
	let centroids = output_tensor(18, DType::F32, &[4])?;
	let counts = output_tensor(19, DType::I32, &[2])?;
	let inputs = [
		NamedTensor::new("points", &points),
		NamedTensor::new("point_indices", &point_indices),
		NamedTensor::new("active_mask", &active_mask),
		NamedTensor::new("assignments", &assignments),
		NamedTensor::new("centroid_count_indices", &count_indices),
		NamedTensor::new("centroid_base", &centroid_base),
		NamedTensor::new("centroid_indices", &centroid_indices),
	];
	let outputs = [
		NamedTensor::new("centroids", &centroids),
		NamedTensor::new("counts", &counts),
	];
	let prepared = parameters(&[
		("points_count", PreparedParameter::U64(3)),
		("dimensions", PreparedParameter::U64(2)),
		("centroid_count", PreparedParameter::U64(2)),
		("points_per_centroid", PreparedParameter::U64(2)),
		("contribution_table_verified", PreparedParameter::Bool(true)),
		(
			"centroid_count_indices_verified",
			PreparedParameter::Bool(true),
		),
		("centroid_base_zero", PreparedParameter::Bool(true)),
		("centroid_indices_unique", PreparedParameter::Bool(true)),
		("assignments_verified", PreparedParameter::Bool(true)),
		("tree_lanes", PreparedParameter::U64(2)),
	]);
	let centroid = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_centroid_update")?,
		&inputs,
		&outputs,
		"points",
		&prepared,
		namespace(110_000),
		ByteCount::new(112),
	))?;
	assert_fragment(&centroid, 112, 3, 7)
}

#[test]
fn materializes_exact_finite_clustering_cases() -> TestResult {
	let points = input_tensor(1, DType::F32, &[4])?;
	let left_indices = input_tensor(2, DType::I32, &[2, 2])?;
	let right_indices = input_tensor(3, DType::I32, &[2, 2])?;
	let rank_indices = input_tensor(4, DType::I32, &[1])?;
	let core_distances = output_tensor(5, DType::F32, &[2, 1])?;
	let inputs = [
		NamedTensor::new("points", &points),
		NamedTensor::new("left_indices", &left_indices),
		NamedTensor::new("right_indices", &right_indices),
		NamedTensor::new("rank_indices", &rank_indices),
	];
	let outputs = [NamedTensor::new("core_distances", &core_distances)];
	let prepared = parameters(&[
		("points_count", PreparedParameter::U64(2)),
		("dimensions", PreparedParameter::U64(2)),
		("minimum_points", PreparedParameter::U64(1)),
		("pair_indices_verified", PreparedParameter::Bool(true)),
		("rank_indices_verified", PreparedParameter::Bool(true)),
		("tree_lanes", PreparedParameter::U64(2)),
	]);
	let core = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_core_distance")?,
		&inputs,
		&outputs,
		"points",
		&prepared,
		namespace(120_000),
		ByteCount::new(72),
	))?;
	assert_fragment(&core, 72, 3, 7)?;

	let points = input_tensor(11, DType::F32, &[2])?;
	let counts = input_tensor(12, DType::I32, &[1])?;
	let row_pointer_base = input_tensor(13, DType::I32, &[2])?;
	let row_pointer_indices = input_tensor(14, DType::I32, &[2])?;
	let row_pointer_updates = input_tensor(15, DType::I32, &[2])?;
	let neighbor_base = input_tensor(16, DType::I32, &[1])?;
	let neighbor_indices = input_tensor(17, DType::I32, &[1])?;
	let row_pointers = output_tensor(18, DType::I32, &[2])?;
	let neighbors = output_tensor(19, DType::I32, &[1])?;
	let inputs = [
		NamedTensor::new("points", &points),
		NamedTensor::new("counts", &counts),
		NamedTensor::new("row_pointer_base", &row_pointer_base),
		NamedTensor::new("row_pointer_indices", &row_pointer_indices),
		NamedTensor::new("row_pointer_updates", &row_pointer_updates),
		NamedTensor::new("neighbor_base", &neighbor_base),
		NamedTensor::new("neighbor_indices", &neighbor_indices),
	];
	let outputs = [
		NamedTensor::new("row_pointers", &row_pointers),
		NamedTensor::new("neighbors", &neighbors),
	];
	let prepared = parameters(&[
		("points_count", PreparedParameter::U64(1)),
		("dimensions", PreparedParameter::U64(2)),
		("epsilon", PreparedParameter::F32Bits(0.5_f32.to_bits())),
		("singleton_tables_verified", PreparedParameter::Bool(true)),
		("bases_zero", PreparedParameter::Bool(true)),
		("destination_indices_unique", PreparedParameter::Bool(true)),
		("tree_lanes", PreparedParameter::U64(1)),
	]);
	let fixed_radius = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_fixed_radius_neighbors")?,
		&inputs,
		&outputs,
		"counts",
		&prepared,
		namespace(130_000),
		ByteCount::new(8),
	))?;
	assert_fragment(&fixed_radius, 8, 3, 4)
}

#[test]
fn materializes_two_node_union_find_and_boruvka() -> TestResult {
	let parent = input_tensor(1, DType::I32, &[2])?;
	let sources = input_tensor(2, DType::I32, &[1])?;
	let destinations = input_tensor(3, DType::I32, &[1])?;
	let node_indices = input_tensor(4, DType::I32, &[2])?;
	let labels_base = input_tensor(5, DType::I32, &[2])?;
	let labels = output_tensor(6, DType::I32, &[2])?;
	let inputs = [
		NamedTensor::new("parent_base", &parent),
		NamedTensor::new("edge_sources", &sources),
		NamedTensor::new("edge_destinations", &destinations),
		NamedTensor::new("node_indices", &node_indices),
		NamedTensor::new("labels_base", &labels_base),
	];
	let outputs = [NamedTensor::new("labels", &labels)];
	let prepared = parameters(&[
		("nodes", PreparedParameter::U64(2)),
		("edges", PreparedParameter::U64(1)),
		("two_node_topology_verified", PreparedParameter::Bool(true)),
		("parent_base_verified", PreparedParameter::Bool(true)),
		("node_indices_verified", PreparedParameter::Bool(true)),
		("labels_base_zero", PreparedParameter::Bool(true)),
	]);
	let union_find = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_union_find_cc")?,
		&inputs,
		&outputs,
		"parent_base",
		&prepared,
		namespace(140_000),
		ByteCount::new(32),
	))?;
	assert_fragment(&union_find, 32, 3, 6)?;

	let parent = input_tensor(11, DType::I32, &[2])?;
	let sources = input_tensor(12, DType::I32, &[2])?;
	let destinations = input_tensor(13, DType::I32, &[2])?;
	let weights = input_tensor(14, DType::F32, &[2])?;
	let edge_indices = input_tensor(15, DType::I32, &[2])?;
	let mask_base = input_tensor(16, DType::I32, &[2])?;
	let unit_update = input_tensor(17, DType::I32, &[1])?;
	let in_mst = output_tensor(18, DType::I32, &[2])?;
	let total_weight = output_tensor(19, DType::F32, &[1])?;
	let inputs = [
		NamedTensor::new("parent", &parent),
		NamedTensor::new("edge_sources", &sources),
		NamedTensor::new("edge_destinations", &destinations),
		NamedTensor::new("edge_weights", &weights),
		NamedTensor::new("edge_indices", &edge_indices),
		NamedTensor::new("mask_base", &mask_base),
		NamedTensor::new("unit_update", &unit_update),
	];
	let outputs = [
		NamedTensor::new("in_mst", &in_mst),
		NamedTensor::new("total_weight", &total_weight),
	];
	let prepared = parameters(&[
		("nodes", PreparedParameter::U64(2)),
		("edges", PreparedParameter::U64(2)),
		("two_node_topology_verified", PreparedParameter::Bool(true)),
		("parent_verified", PreparedParameter::Bool(true)),
		("edge_indices_verified", PreparedParameter::Bool(true)),
		("edge_weights_finite", PreparedParameter::Bool(true)),
		("mask_base_zero", PreparedParameter::Bool(true)),
		("tree_lanes", PreparedParameter::U64(2)),
	]);
	let boruvka = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_boruvka_mst")?,
		&inputs,
		&outputs,
		"parent",
		&prepared,
		namespace(150_000),
		ByteCount::new(32),
	))?;
	assert_fragment(&boruvka, 32, 3, 6)
}

#[test]
fn rejects_recurrences_and_data_dependent_shapes_without_exact_static_graphs() -> TestResult {
	let rewards = input_tensor(1, DType::F32, &[2])?;
	let returns = output_tensor(2, DType::F32, &[2])?;
	let inputs = [NamedTensor::new("rewards", &rewards)];
	let outputs = [NamedTensor::new("returns", &returns)];
	let prepared = parameters(&[
		("length", PreparedParameter::U64(2)),
		("gamma", PreparedParameter::F32Bits(0.9_f32.to_bits())),
		("tree_lanes", PreparedParameter::U64(2)),
	]);
	let error = materialize_composition(MaterializationRequest::new(
		operation_registry().resolve_unique("gpu_discounted_returns")?,
		&inputs,
		&outputs,
		"rewards",
		&prepared,
		namespace(160_000),
		ByteCount::new(8),
	))
	.expect_err("non-unit discount unexpectedly became a plain sum scan");
	assert_eq!(error.kind, OperationErrorKind::UnsupportedConcreteShape);
	Ok(())
}
