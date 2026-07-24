use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;

use recipe_core::{AliasPermission, ByteCount, DType, KernelTemplateId, ValueId};
use recipe_language::{CalculationGraph, PrimitiveKind, Reduce, Scan, Shape, SortDirection, Tensor};
use recipe_ops::{
	BinaryClassificationMetricRequest, IdentityNamespace, OperationErrorKind, RecallAtOutput,
	append_binary_classification_metrics, binary_metric_requirements, materialize_binary_classification_metrics,
};
use recipe_primitives::lower;

type TestResult = Result<(), Box<dyn Error>>;

fn tensor(
	id: u64,
	dtype: DType,
	extents: &[u64],
	external_input: bool,
	external_output: bool,
) -> Result<Tensor, recipe_language::LanguageError> {
	Tensor::contiguous(
		ValueId::new(id),
		dtype,
		Shape::new(extents.to_vec())?,
		external_input,
		external_output,
	)
}

fn request(examples: u64) -> Result<BinaryClassificationMetricRequest, recipe_language::LanguageError> {
	Ok(BinaryClassificationMetricRequest {
		probabilities: tensor(1, DType::F32, &[examples], true, false)?,
		targets: tensor(2, DType::F32, &[examples], true, false)?,
		per_element_bce: tensor(3, DType::F32, &[examples], true, false)?,
		mean_bce: tensor(10, DType::F32, &[1], false, true)?,
		auroc: tensor(11, DType::F32, &[1], false, true)?,
		auprc: tensor(12, DType::F32, &[1], false, true)?,
		brier_score: tensor(13, DType::F32, &[1], false, true)?,
		expected_calibration_error: tensor(14, DType::F32, &[1], false, true)?,
		recall_at: vec![
			RecallAtOutput::new(0.25, tensor(15, DType::F32, &[1], false, true)?),
			RecallAtOutput::new(0.75, tensor(16, DType::F32, &[1], false, true)?),
		],
		calibration_bins: 4,
		tree_lanes: 64,
		identity_namespace: IdentityNamespace::new(ValueId::new(1_000), 128, KernelTemplateId::new(2_000), 128),
		workspace_limit: ByteCount::new(1 << 20),
	})
}

fn boundaries(request: &BinaryClassificationMetricRequest) -> Vec<Tensor> {
	let mut tensors = vec![
		request.probabilities.clone(),
		request.targets.clone(),
		request.per_element_bce.clone(),
		request.mean_bce.clone(),
		request.auroc.clone(),
		request.auprc.clone(),
		request.brier_score.clone(),
		request.expected_calibration_error.clone(),
	];
	tensors.extend(request.recall_at.iter().map(|recall| recall.output.clone()));
	tensors
}

#[test]
fn materializes_every_binary_metric_as_lowerable_gpu_primitives() -> TestResult {
	let request = request(8)?;
	let requirements = binary_metric_requirements(8, 2, 4)?;
	assert_eq!(requirements.intermediate_values, 48);
	assert_eq!(requirements.kernels, 46);
	assert_eq!(requirements.workspace_bytes, ByteCount::new(1_004));

	let materialized = materialize_binary_classification_metrics(&request)?;
	assert_eq!(materialized.intermediate_values.len(), 48);
	assert_eq!(materialized.kernels.len(), 46);
	assert_eq!(materialized.workspace_bytes, requirements.workspace_bytes);
	materialized.graph.validate()?;

	let tensors = materialized
		.graph
		.tensors
		.iter()
		.map(|tensor| (tensor.id, tensor))
		.collect::<BTreeMap<_, _>>();
	for node in &materialized.graph.nodes {
		let lowered = lower(&node.kernel, &tensors)?;
		lowered.validate().map_err(|errors| {
			errors.into_iter()
				.map(|error| error.to_string())
				.collect::<Vec<_>>()
				.join("; ")
		})?;
		assert!(
			node.kernel
				.alias_rules
				.iter()
				.all(|rule| rule.permission == AliasPermission::Forbidden)
		);
	}

	let sorts = materialized
		.graph
		.nodes
		.iter()
		.filter_map(|node| match &node.kernel.kind {
			PrimitiveKind::Sort(sort) => Some(sort),
			_ => None,
		})
		.collect::<Vec<_>>();
	assert_eq!(sorts.len(), 1);
	assert_eq!(sorts[0].direction, SortDirection::Descending);
	assert!(sorts[0].stable);
	assert!(sorts[0].emit_indices);

	let histograms = materialized
		.graph
		.nodes
		.iter()
		.filter_map(|node| match &node.kernel.kind {
			PrimitiveKind::Histogram(histogram) => Some(histogram),
			_ => None,
		})
		.collect::<Vec<_>>();
	assert_eq!(histograms.len(), 2);
	assert_eq!(
		histograms
			.iter()
			.filter(|histogram| histogram.weighted)
			.count(),
		1
	);
	assert!(
		materialized
			.graph
			.nodes
			.iter()
			.filter_map(|node| match &node.kernel.kind {
				PrimitiveKind::Reduce(Reduce { tree_lanes, .. })
				| PrimitiveKind::Scan(Scan { tree_lanes, .. }) => Some(*tree_lanes),
				_ => None,
			})
			.all(|lanes| lanes == 64)
	);
	assert!(!materialized.graph.nodes.iter().any(|node| {
		matches!(
			node.kernel.kind,
			PrimitiveKind::Random(_) | PrimitiveKind::Scatter(_) | PrimitiveKind::Contraction(_)
		)
	}));
	Ok(())
}

#[test]
fn append_preserves_boundary_tensors_and_completes_the_graph() -> TestResult {
	let request = request(8)?;
	let boundary_ids = boundaries(&request)
		.iter()
		.map(|tensor| tensor.id)
		.collect::<BTreeSet<_>>();
	let mut graph = CalculationGraph {
		tensors: boundaries(&request),
		nodes: Vec::new(),
	};
	let materialized = append_binary_classification_metrics(&mut graph, &request)?;
	graph.validate()?;
	assert_eq!(
		graph.tensors.len(),
		boundary_ids.len() + materialized.intermediate_values.len()
	);
	assert!(boundary_ids.iter().all(|id| {
		graph.tensors
			.iter()
			.filter(|tensor| tensor.id == *id)
			.count() == 1
	}));
	let produced = graph
		.nodes
		.iter()
		.flat_map(|node| node.kernel.outputs.iter().copied())
		.collect::<BTreeSet<_>>();
	for output in [
		request.mean_bce.id,
		request.auroc.id,
		request.auprc.id,
		request.brier_score.id,
		request.expected_calibration_error.id,
		request.recall_at[0].output.id,
		request.recall_at[1].output.id,
	] {
		assert!(produced.contains(&output));
	}
	Ok(())
}

#[test]
fn rejects_ambiguous_or_under_reserved_static_contracts() -> TestResult {
	let mut invalid_threshold = request(8)?;
	invalid_threshold.recall_at[0].threshold_bits = f32::NAN.to_bits();
	let error = materialize_binary_classification_metrics(&invalid_threshold).unwrap_err();
	assert_eq!(
		error.kind,
		OperationErrorKind::InvalidMaterializationRequest
	);
	assert!(error.detail.contains("finite f32"));

	let mut duplicate_threshold = request(8)?;
	duplicate_threshold.recall_at[1].threshold_bits = duplicate_threshold.recall_at[0].threshold_bits;
	let error = materialize_binary_classification_metrics(&duplicate_threshold).unwrap_err();
	assert_eq!(
		error.kind,
		OperationErrorKind::InvalidMaterializationRequest
	);
	assert!(error.detail.contains("duplicates"));

	let mut insufficient = request(8)?;
	insufficient.identity_namespace =
		IdentityNamespace::new(ValueId::new(1_000), 47, KernelTemplateId::new(2_000), 46);
	let error = materialize_binary_classification_metrics(&insufficient).unwrap_err();
	assert_eq!(error.kind, OperationErrorKind::IdentityNamespaceExhausted);
	assert!(error.detail.contains("48 intermediate"));

	let mut no_bins = request(8)?;
	no_bins.calibration_bins = 0;
	let error = materialize_binary_classification_metrics(&no_bins).unwrap_err();
	assert_eq!(error.kind, OperationErrorKind::UnsupportedConcreteShape);
	assert!(error.detail.contains("calibration bin"));
	Ok(())
}
