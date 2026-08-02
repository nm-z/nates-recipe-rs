use std::collections::BTreeSet;

use recipe_core::{AliasPermission, ByteCount, DType, KernelTemplateId, ScalarOpcode, ValueId};
use recipe_language::{
	AtomicOrdering, AxisSet, CalculationGraph, CalculationNode, Elementwise, Gather, Histogram, IndexBounds,
	IndexMap, PrimitiveAliasRule, PrimitiveKernel, PrimitiveKind, Reduce, ReduceOperator, ReduceResult,
	ScalarExpression, ScalarProgramBuilder, Scan, ScanMode, Shape, Sort, SortDirection, Tensor,
};

use crate::{IdentityNamespace, OperationError, OperationErrorKind, OperationResult};

/// Largest population whose integer counts remain exact binary32 values while
/// also respecting Recipe's "fewer than seven significant figures" contract.
pub const MAX_EXACT_BINARY_EXAMPLES: u64 = 9_999_999;
pub const MAX_CALIBRATION_BINS: u32 = 256;
pub const MAX_RECALL_THRESHOLDS: usize = 256;

/// One statically requested recall threshold and its scalar output tensor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecallAtOutput {
	pub threshold_bits: u32,
	pub output: Tensor,
}

impl RecallAtOutput {
	#[must_use]
	pub fn new(threshold: f32, output: Tensor) -> Self {
		Self {
			threshold_bits: threshold.to_bits(),
			output,
		}
	}

	#[must_use]
	pub const fn threshold(&self) -> f32 { f32::from_bits(self.threshold_bits) }
}

/// Fully typed boundary for GPU-only binary-classification metrics.
///
/// `per_element_bce` is the stable loss vector already emitted by the training
/// operation. Every result is a caller-declared `[1]` f32 tensor. AUPRC has the
/// precise, tie-aware non-interpolated average-precision definition. Expected
/// calibration error uses equal-width bins over `[0, 1]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BinaryClassificationMetricRequest {
	pub probabilities: Tensor,
	pub targets: Tensor,
	pub per_element_bce: Tensor,
	pub mean_bce: Tensor,
	pub auroc: Tensor,
	pub auprc: Tensor,
	pub brier_score: Tensor,
	pub expected_calibration_error: Tensor,
	pub recall_at: Vec<RecallAtOutput>,
	pub calibration_bins: u32,
	pub tree_lanes: u32,
	pub identity_namespace: IdentityNamespace,
	pub workspace_limit: ByteCount,
}

/// Exact static resources needed before identities or GPU memory are reserved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BinaryMetricRequirements {
	pub intermediate_values: u64,
	pub kernels: u64,
	pub workspace_bytes: ByteCount,
}

/// A validated graph fragment plus its exact identity and workspace footprint.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BinaryMetricMaterialization {
	pub graph: CalculationGraph,
	pub intermediate_values: Vec<ValueId>,
	pub kernels: Vec<KernelTemplateId>,
	pub workspace_bytes: ByteCount,
	pub identity_namespace: IdentityNamespace,
}

/// Return the exact graph-resource reservation for a fixed metric population.
pub fn binary_metric_requirements(
	examples: u64,
	recall_thresholds: usize,
	calibration_bins: u32,
) -> OperationResult<BinaryMetricRequirements> {
	validate_dimensions(examples, recall_thresholds, calibration_bins)?;
	let recalls = u64::try_from(recall_thresholds).map_err(|error| {
		metric_error(
			OperationErrorKind::WorkspaceArithmeticOverflow,
			format!("recall threshold count does not fit u64: {error}"),
		)
	})?;
	let bins = u64::from(calibration_bins);
	let intermediate_values = checked_add(
		checked_add(
			24,
			checked_mul(2, recalls, "recall intermediate identities")?,
			"base and recall intermediate identities",
		)?,
		checked_mul(5, bins, "calibration intermediate identities")?,
		"metric intermediate identities",
	)?;
	let kernels = checked_add(
		checked_add(
			24,
			checked_mul(3, recalls, "recall kernel identities")?,
			"base and recall kernel identities",
		)?,
		checked_mul(4, bins, "calibration kernel identities")?,
		"metric kernel identities",
	)?;
	let vector_bytes = checked_mul(
		examples,
		u64::from(DType::F32.byte_width()),
		"metric vector bytes",
	)?;
	let base_workspace = checked_add(
		checked_mul(19, vector_bytes, "base metric vector workspace")?,
		20,
		"base metric workspace",
	)?;
	let recall_workspace = checked_mul(
		recalls,
		checked_add(vector_bytes, 4, "one recall workspace")?,
		"recall workspace",
	)?;
	let calibration_workspace = checked_mul(
		bins,
		checked_add(
			checked_mul(2, vector_bytes, "one calibration bin vector workspace")?,
			12,
			"one calibration bin workspace",
		)?,
		"calibration workspace",
	)?;
	let workspace_bytes = checked_add(
		checked_add(
			base_workspace,
			recall_workspace,
			"base and recall workspace",
		)?,
		calibration_workspace,
		"metric workspace",
	)?;
	Ok(BinaryMetricRequirements {
		intermediate_values,
		kernels,
		workspace_bytes: ByteCount::new(workspace_bytes),
	})
}

/// Materialize a self-validating calculation-graph fragment.
///
/// Boundary flags on the supplied tensors are normalized for the fragment:
/// metric inputs become external inputs and metric results become external
/// outputs. Payload calculation remains entirely in Recipe primitives.
pub fn materialize_binary_classification_metrics(
	request: &BinaryClassificationMetricRequest,
) -> OperationResult<BinaryMetricMaterialization> {
	let examples = validate_request(request)?;
	let requirements = binary_metric_requirements(examples, request.recall_at.len(), request.calibration_bins)?;
	if requirements.intermediate_values > request.identity_namespace.value_capacity() {
		return Err(metric_error(
			OperationErrorKind::IdentityNamespaceExhausted,
			format!(
				"binary metrics need {} intermediate identities, namespace reserves {}",
				requirements.intermediate_values,
				request.identity_namespace.value_capacity()
			),
		));
	}
	if requirements.kernels > request.identity_namespace.kernel_capacity() {
		return Err(metric_error(
			OperationErrorKind::IdentityNamespaceExhausted,
			format!(
				"binary metrics need {} kernel identities, namespace reserves {}",
				requirements.kernels,
				request.identity_namespace.kernel_capacity()
			),
		));
	}
	if requirements.workspace_bytes.get() > request.workspace_limit.get() {
		return Err(metric_error(
			OperationErrorKind::WorkspaceLimitExceeded,
			format!(
				"binary metrics need {} workspace bytes, limit is {}",
				requirements.workspace_bytes.get(),
				request.workspace_limit.get()
			),
		));
	}

	let mut emitter = MetricEmitter::new(request, requirements)?;
	let vector_shape = request.probabilities.shape.clone();
	let scalar_shape = scalar_shape()?;

	// One guarded copy validates the entire public metric boundary on device.
	let valid_probabilities = emitter.intermediate(DType::F32, vector_shape.clone())?;
	let valid_targets = emitter.intermediate(DType::F32, vector_shape.clone())?;
	let valid_losses = emitter.intermediate(DType::F32, vector_shape.clone())?;
	emitter.emit(
		vec![
			request.probabilities.id,
			request.targets.id,
			request.per_element_bce.id,
		],
		vec![valid_probabilities, valid_targets, valid_losses],
		elementwise(validated_inputs_program()?),
	)?;

	let loss_sum = emitter.intermediate(DType::F32, scalar_shape.clone())?;
	emitter.emit(
		vec![valid_losses],
		vec![loss_sum],
		sum_reduction(0, request.tree_lanes)?,
	)?;
	emitter.emit(
		vec![loss_sum],
		vec![request.mean_bce.id],
		elementwise(divide_by_constant_program(examples as f32)?),
	)?;

	let brier_elements = emitter.intermediate(DType::F32, vector_shape.clone())?;
	emitter.emit(
		vec![valid_probabilities, valid_targets],
		vec![brier_elements],
		elementwise(brier_program()?),
	)?;
	let brier_sum = emitter.intermediate(DType::F32, scalar_shape.clone())?;
	emitter.emit(
		vec![brier_elements],
		vec![brier_sum],
		sum_reduction(0, request.tree_lanes)?,
	)?;
	emitter.emit(
		vec![brier_sum],
		vec![request.brier_score.id],
		elementwise(divide_by_constant_program(examples as f32)?),
	)?;

	let positive_count = emitter.intermediate(DType::F32, scalar_shape.clone())?;
	emitter.emit(
		vec![valid_targets],
		vec![positive_count],
		sum_reduction(0, request.tree_lanes)?,
	)?;

	// Stable descending sort plus explicit tie groups yields exact rank metrics
	// without an O(n^2) pair matrix.
	let sorted_probabilities = emitter.intermediate(DType::F32, vector_shape.clone())?;
	let sorted_indices = emitter.intermediate(DType::I32, vector_shape.clone())?;
	emitter.emit(
		vec![valid_probabilities],
		vec![sorted_probabilities, sorted_indices],
		PrimitiveKind::Sort(Sort {
			axis: 0,
			direction: SortDirection::Descending,
			stable: true,
			emit_indices: true,
		}),
	)?;
	let sorted_targets = emitter.intermediate(DType::F32, vector_shape.clone())?;
	emitter.emit(
		vec![valid_targets, sorted_indices],
		vec![sorted_targets],
		gather(IndexBounds::Reject),
	)?;

	let positions = emitter.intermediate(DType::I32, vector_shape.clone())?;
	emitter.emit(
		Vec::new(),
		vec![positions],
		PrimitiveKind::IndexMap(IndexMap {
			start: 0,
			element_step: 1,
			iteration_step: 0,
			modulus: None,
		}),
	)?;
	let previous_indices = emitter.intermediate(DType::I32, vector_shape.clone())?;
	emitter.emit(
		Vec::new(),
		vec![previous_indices],
		PrimitiveKind::IndexMap(IndexMap {
			start: -1,
			element_step: 1,
			iteration_step: 0,
			modulus: None,
		}),
	)?;
	let previous_probabilities = emitter.intermediate(DType::F32, vector_shape.clone())?;
	emitter.emit(
		vec![sorted_probabilities, previous_indices],
		vec![previous_probabilities],
		gather(IndexBounds::Clamp),
	)?;
	let group_starts = emitter.intermediate(DType::I32, vector_shape.clone())?;
	emitter.emit(
		vec![sorted_probabilities, previous_probabilities, positions],
		vec![group_starts],
		elementwise(group_start_program()?),
	)?;
	let one_based_group_ids = emitter.intermediate(DType::I32, vector_shape.clone())?;
	emitter.emit(
		vec![group_starts],
		vec![one_based_group_ids],
		inclusive_sum_scan(request.tree_lanes),
	)?;
	let group_ids = emitter.intermediate(DType::I32, vector_shape.clone())?;
	emitter.emit(
		vec![one_based_group_ids],
		vec![group_ids],
		elementwise(subtract_one_program()?),
	)?;

	let group_counts = emitter.intermediate(DType::I32, vector_shape.clone())?;
	emitter.emit(
		vec![group_ids],
		vec![group_counts],
		PrimitiveKind::Histogram(Histogram {
			bins: u32::try_from(examples).expect("validated population fits u32"),
			weighted: false,
			ordering: AtomicOrdering::SequentiallyConsistent,
		}),
	)?;
	let group_positives = emitter.intermediate(DType::F32, vector_shape.clone())?;
	emitter.emit(
		vec![group_ids, sorted_targets],
		vec![group_positives],
		PrimitiveKind::Histogram(Histogram {
			bins: u32::try_from(examples).expect("validated population fits u32"),
			weighted: true,
			ordering: AtomicOrdering::SequentiallyConsistent,
		}),
	)?;
	let cumulative_counts = emitter.intermediate(DType::I32, vector_shape.clone())?;
	emitter.emit(
		vec![group_counts],
		vec![cumulative_counts],
		inclusive_sum_scan(request.tree_lanes),
	)?;
	let cumulative_positives = emitter.intermediate(DType::F32, vector_shape.clone())?;
	emitter.emit(
		vec![group_positives],
		vec![cumulative_positives],
		inclusive_sum_scan(request.tree_lanes),
	)?;
	let auroc_contributions = emitter.intermediate(DType::F32, vector_shape.clone())?;
	let auprc_contributions = emitter.intermediate(DType::F32, vector_shape.clone())?;
	emitter.emit(
		vec![
			group_counts,
			group_positives,
			cumulative_counts,
			cumulative_positives,
			positive_count,
		],
		vec![auroc_contributions, auprc_contributions],
		elementwise(ranking_contributions_program(examples as f32)?),
	)?;
	let auroc_numerator = emitter.intermediate(DType::F32, scalar_shape.clone())?;
	emitter.emit(
		vec![auroc_contributions],
		vec![auroc_numerator],
		sum_reduction(0, request.tree_lanes)?,
	)?;
	let auprc_numerator = emitter.intermediate(DType::F32, scalar_shape.clone())?;
	emitter.emit(
		vec![auprc_contributions],
		vec![auprc_numerator],
		sum_reduction(0, request.tree_lanes)?,
	)?;
	emitter.emit(
		vec![positive_count, auroc_numerator, auprc_numerator],
		vec![request.auroc.id, request.auprc.id],
		elementwise(normalize_ranking_program(examples as f32)?),
	)?;

	for recall in &request.recall_at {
		let hits = emitter.intermediate(DType::F32, vector_shape.clone())?;
		emitter.emit(
			vec![valid_probabilities, valid_targets],
			vec![hits],
			elementwise(recall_hits_program(recall.threshold())?),
		)?;
		let true_positives = emitter.intermediate(DType::F32, scalar_shape.clone())?;
		emitter.emit(
			vec![hits],
			vec![true_positives],
			sum_reduction(0, request.tree_lanes)?,
		)?;
		emitter.emit(
			vec![true_positives, positive_count],
			vec![recall.output.id],
			elementwise(normalize_positive_count_program()?),
		)?;
	}

	let mut calibration_contributions = Vec::with_capacity(request.calibration_bins as usize);
	for bin in 0..request.calibration_bins {
		let lower = bin as f32 / request.calibration_bins as f32;
		let upper = (bin + 1) as f32 / request.calibration_bins as f32;
		let confidence_members = emitter.intermediate(DType::F32, vector_shape.clone())?;
		let target_members = emitter.intermediate(DType::F32, vector_shape.clone())?;
		emitter.emit(
			vec![valid_probabilities, valid_targets],
			vec![confidence_members, target_members],
			elementwise(calibration_bin_program(
				lower,
				upper,
				bin + 1 == request.calibration_bins,
			)?),
		)?;
		let confidence_sum = emitter.intermediate(DType::F32, scalar_shape.clone())?;
		emitter.emit(
			vec![confidence_members],
			vec![confidence_sum],
			sum_reduction(0, request.tree_lanes)?,
		)?;
		let target_sum = emitter.intermediate(DType::F32, scalar_shape.clone())?;
		emitter.emit(
			vec![target_members],
			vec![target_sum],
			sum_reduction(0, request.tree_lanes)?,
		)?;
		let contribution = emitter.intermediate(DType::F32, scalar_shape.clone())?;
		emitter.emit(
			vec![confidence_sum, target_sum],
			vec![contribution],
			elementwise(calibration_contribution_program(examples as f32)?),
		)?;
		calibration_contributions.push(contribution);
	}
	emitter.emit(
		calibration_contributions,
		vec![request.expected_calibration_error.id],
		elementwise(fixed_sum_program(request.calibration_bins as usize)?),
	)?;

	emitter.finish()
}

/// Append the validated fragment to an in-progress calculation graph.
///
/// The caller graph must already declare all request boundary tensors. Only
/// intermediates and kernels are appended, so tensor identities stay canonical.
/// The completed graph should be validated after all independently compiled
/// fragments have been appended.
pub fn append_binary_classification_metrics(
	graph: &mut CalculationGraph,
	request: &BinaryClassificationMetricRequest,
) -> OperationResult<BinaryMetricMaterialization> {
	validate_boundary_storage(graph, request)?;
	let materialized = materialize_binary_classification_metrics(request)?;
	let existing_values = graph
		.tensors
		.iter()
		.map(|tensor| tensor.id)
		.collect::<BTreeSet<_>>();
	if let Some(value) = materialized
		.intermediate_values
		.iter()
		.find(|value| existing_values.contains(value))
	{
		return Err(metric_error(
			OperationErrorKind::IdentityNamespaceOverlap,
			format!("binary metric intermediate {value} already exists in caller graph"),
		));
	}
	let existing_kernels = graph
		.nodes
		.iter()
		.map(|node| node.kernel.id)
		.collect::<BTreeSet<_>>();
	if let Some(kernel) = materialized
		.kernels
		.iter()
		.find(|kernel| existing_kernels.contains(kernel))
	{
		return Err(metric_error(
			OperationErrorKind::IdentityNamespaceOverlap,
			format!("binary metric kernel {kernel} already exists in caller graph"),
		));
	}
	let output_ids = output_tensors(request)
		.into_iter()
		.map(|tensor| tensor.id)
		.collect::<BTreeSet<_>>();
	if let Some(node) = graph.nodes.iter().find(|node| {
		node.kernel
			.outputs
			.iter()
			.any(|output| output_ids.contains(output))
	}) {
		return Err(metric_error(
			OperationErrorKind::GraphMaterializationFailed,
			format!(
				"caller graph kernel {} already produces a binary metric output",
				node.kernel.id
			),
		));
	}
	let intermediate_ids = materialized
		.intermediate_values
		.iter()
		.copied()
		.collect::<BTreeSet<_>>();
	graph.tensors.extend(
		materialized
			.graph
			.tensors
			.iter()
			.filter(|tensor| intermediate_ids.contains(&tensor.id))
			.cloned(),
	);
	graph.nodes.extend(materialized.graph.nodes.iter().cloned());
	Ok(materialized)
}

struct MetricEmitter {
	tensors: Vec<Tensor>,
	nodes: Vec<CalculationNode>,
	intermediate_values: Vec<ValueId>,
	kernels: Vec<KernelTemplateId>,
	next_value: u64,
	value_end: u64,
	next_kernel: u64,
	kernel_end: u64,
	workspace_bytes: u64,
	requirements: BinaryMetricRequirements,
	identity_namespace: IdentityNamespace,
}

impl MetricEmitter {
	fn new(
		request: &BinaryClassificationMetricRequest,
		requirements: BinaryMetricRequirements,
	) -> OperationResult<Self> {
		let first_value = request.identity_namespace.first_value().get();
		let value_end = first_value
			.checked_add(request.identity_namespace.value_capacity())
			.ok_or_else(|| {
				metric_error(
					OperationErrorKind::IdentityNamespaceExhausted,
					"binary metric value namespace overflows u64",
				)
			})?;
		let first_kernel = request.identity_namespace.first_kernel().get();
		let kernel_end = first_kernel
			.checked_add(request.identity_namespace.kernel_capacity())
			.ok_or_else(|| {
				metric_error(
					OperationErrorKind::IdentityNamespaceExhausted,
					"binary metric kernel namespace overflows u64",
				)
			})?;
		let mut tensors = Vec::new();
		for tensor in input_tensors(request) {
			let mut tensor = tensor.clone();
			tensor.external_input = true;
			tensor.external_output = false;
			tensors.push(tensor);
		}
		for tensor in output_tensors(request) {
			let mut tensor = tensor.clone();
			tensor.external_input = false;
			tensor.external_output = true;
			tensors.push(tensor);
		}
		if let Some(tensor) = tensors
			.iter()
			.find(|tensor| tensor.id.get() >= first_value && tensor.id.get() < value_end)
		{
			return Err(metric_error(
				OperationErrorKind::IdentityNamespaceOverlap,
				format!(
					"boundary tensor {} overlaps binary metric intermediate range [{first_value}, {value_end})",
					tensor.id
				),
			));
		}
		Ok(Self {
			tensors,
			nodes: Vec::new(),
			intermediate_values: Vec::with_capacity(requirements.intermediate_values as usize),
			kernels: Vec::with_capacity(requirements.kernels as usize),
			next_value: first_value,
			value_end,
			next_kernel: first_kernel,
			kernel_end,
			workspace_bytes: 0,
			requirements,
			identity_namespace: request.identity_namespace,
		})
	}

	fn intermediate(&mut self, dtype: DType, shape: Shape) -> OperationResult<ValueId> {
		if self.next_value == self.value_end {
			return Err(metric_error(
				OperationErrorKind::IdentityNamespaceExhausted,
				"binary metric intermediate identity range is exhausted",
			));
		}
		let value = ValueId::new(self.next_value);
		self.next_value += 1;
		let tensor = Tensor::contiguous(value, dtype, shape, false, false).map_err(|error| {
			metric_error(
				OperationErrorKind::GraphMaterializationFailed,
				error.to_string(),
			)
		})?;
		self.workspace_bytes = self
			.workspace_bytes
			.checked_add(tensor.storage_bytes.get())
			.ok_or_else(|| {
				metric_error(
					OperationErrorKind::WorkspaceArithmeticOverflow,
					"binary metric workspace total overflows u64",
				)
			})?;
		self.intermediate_values.push(value);
		self.tensors.push(tensor);
		Ok(value)
	}

	fn emit(&mut self, inputs: Vec<ValueId>, outputs: Vec<ValueId>, kind: PrimitiveKind) -> OperationResult<()> {
		if self.next_kernel == self.kernel_end {
			return Err(metric_error(
				OperationErrorKind::IdentityNamespaceExhausted,
				"binary metric kernel identity range is exhausted",
			));
		}
		let id = KernelTemplateId::new(self.next_kernel);
		self.next_kernel += 1;
		self.kernels.push(id);
		self.nodes.push(CalculationNode {
			kernel: PrimitiveKernel {
				id,
				alias_rules: forbidden_aliases(inputs.len(), outputs.len()),
				inputs,
				outputs,
				kind,
			},
		});
		Ok(())
	}

	fn finish(self) -> OperationResult<BinaryMetricMaterialization> {
		if self.intermediate_values.len() as u64 != self.requirements.intermediate_values {
			return Err(metric_error(
				OperationErrorKind::GraphMaterializationFailed,
				format!(
					"binary metric materializer emitted {} intermediates, reservation requires {}",
					self.intermediate_values.len(),
					self.requirements.intermediate_values
				),
			));
		}
		if self.kernels.len() as u64 != self.requirements.kernels {
			return Err(metric_error(
				OperationErrorKind::GraphMaterializationFailed,
				format!(
					"binary metric materializer emitted {} kernels, reservation requires {}",
					self.kernels.len(),
					self.requirements.kernels
				),
			));
		}
		if self.workspace_bytes != self.requirements.workspace_bytes.get() {
			return Err(metric_error(
				OperationErrorKind::WorkspaceFormulaMismatch,
				format!(
					"binary metric materializer emitted {} workspace bytes, formula requires {}",
					self.workspace_bytes,
					self.requirements.workspace_bytes.get()
				),
			));
		}
		let graph = CalculationGraph {
			tensors: self.tensors,
			nodes: self.nodes,
		};
		graph.validate().map_err(|error| {
			metric_error(
				OperationErrorKind::GraphMaterializationFailed,
				error.to_string(),
			)
		})?;
		Ok(BinaryMetricMaterialization {
			graph,
			intermediate_values: self.intermediate_values,
			kernels: self.kernels,
			workspace_bytes: ByteCount::new(self.workspace_bytes),
			identity_namespace: self.identity_namespace,
		})
	}
}

fn validate_request(request: &BinaryClassificationMetricRequest) -> OperationResult<u64> {
	for tensor in input_tensors(request)
		.into_iter()
		.chain(output_tensors(request))
	{
		tensor.validate().map_err(|error| {
			metric_error(
				OperationErrorKind::InvalidMaterializationRequest,
				error.to_string(),
			)
		})?;
	}
	for (name, tensor) in [
		("probabilities", &request.probabilities),
		("targets", &request.targets),
		("per_element_bce", &request.per_element_bce),
	] {
		require_dtype(name, tensor, DType::F32)?;
		if tensor.shape.rank() != 1 {
			return Err(metric_error(
				OperationErrorKind::UnsupportedConcreteShape,
				format!("{name} must be a rank-one vector"),
			));
		}
	}
	if request.targets.shape != request.probabilities.shape
		|| request.per_element_bce.shape != request.probabilities.shape
	{
		return Err(metric_error(
			OperationErrorKind::UnsupportedConcreteShape,
			"probabilities, targets, and per-element BCE must have identical shapes",
		));
	}
	let examples = request.probabilities.shape.elements();
	validate_dimensions(examples, request.recall_at.len(), request.calibration_bins)?;
	if request.tree_lanes == 0 || request.tree_lanes > 1024 || !request.tree_lanes.is_power_of_two() {
		return Err(metric_error(
			OperationErrorKind::InvalidMaterializationRequest,
			"tree_lanes must be a power of two in 1..=1024",
		));
	}
	for (name, tensor) in [
		("mean_bce", &request.mean_bce),
		("auroc", &request.auroc),
		("auprc", &request.auprc),
		("brier_score", &request.brier_score),
		(
			"expected_calibration_error",
			&request.expected_calibration_error,
		),
	] {
		require_scalar_output(name, tensor)?;
	}
	let mut thresholds = BTreeSet::new();
	for (index, recall) in request.recall_at.iter().enumerate() {
		require_scalar_output(&format!("recall_at[{index}]"), &recall.output)?;
		let threshold = recall.threshold();
		if !threshold.is_finite() || !(0.0..=1.0).contains(&threshold) {
			return Err(metric_error(
				OperationErrorKind::InvalidMaterializationRequest,
				format!("recall_at[{index}] threshold must be a finite f32 in [0, 1]"),
			));
		}
		if !thresholds.insert(recall.threshold_bits) {
			return Err(metric_error(
				OperationErrorKind::InvalidMaterializationRequest,
				format!("recall_at[{index}] duplicates an earlier threshold bit pattern"),
			));
		}
	}
	let mut boundaries = BTreeSet::new();
	for tensor in input_tensors(request)
		.into_iter()
		.chain(output_tensors(request))
	{
		if !boundaries.insert(tensor.id) {
			return Err(metric_error(
				OperationErrorKind::InvalidMaterializationRequest,
				format!("binary metric boundary repeats tensor {}", tensor.id),
			));
		}
	}
	Ok(examples)
}

fn validate_dimensions(examples: u64, recall_thresholds: usize, calibration_bins: u32) -> OperationResult<()> {
	if examples == 0 || examples > MAX_EXACT_BINARY_EXAMPLES {
		return Err(metric_error(
			OperationErrorKind::UnsupportedConcreteShape,
			format!("binary metric population must be in 1..={MAX_EXACT_BINARY_EXAMPLES}, received {examples}"),
		));
	}
	if recall_thresholds > MAX_RECALL_THRESHOLDS {
		return Err(metric_error(
			OperationErrorKind::UnsupportedConcreteShape,
			format!("binary metrics support at most {MAX_RECALL_THRESHOLDS} static recall thresholds"),
		));
	}
	if calibration_bins == 0 || calibration_bins > MAX_CALIBRATION_BINS {
		return Err(metric_error(
			OperationErrorKind::UnsupportedConcreteShape,
			format!("calibration bin count must be in 1..={MAX_CALIBRATION_BINS}, received {calibration_bins}"),
		));
	}
	Ok(())
}

fn validate_boundary_storage(
	graph: &CalculationGraph,
	request: &BinaryClassificationMetricRequest,
) -> OperationResult<()> {
	let mut ids = BTreeSet::new();
	for tensor in &graph.tensors {
		if !ids.insert(tensor.id) {
			return Err(metric_error(
				OperationErrorKind::GraphMaterializationFailed,
				format!("caller graph repeats tensor {}", tensor.id),
			));
		}
	}
	for declared in input_tensors(request)
		.into_iter()
		.chain(output_tensors(request))
	{
		let existing = graph
			.tensors
			.iter()
			.find(|tensor| tensor.id == declared.id)
			.ok_or_else(|| {
				metric_error(
					OperationErrorKind::InvalidMaterializationRequest,
					format!(
						"caller graph does not declare boundary tensor {}",
						declared.id
					),
				)
			})?;
		if !same_storage_contract(existing, declared) {
			return Err(metric_error(
				OperationErrorKind::InvalidMaterializationRequest,
				format!(
					"caller graph tensor {} differs from the binary metric storage contract",
					declared.id
				),
			));
		}
	}
	Ok(())
}

fn same_storage_contract(left: &Tensor, right: &Tensor) -> bool {
	left.id == right.id
		&& left.dtype == right.dtype
		&& left.shape == right.shape
		&& left.layout == right.layout
		&& left.storage_bytes == right.storage_bytes
}

fn input_tensors(request: &BinaryClassificationMetricRequest) -> Vec<&Tensor> {
	vec![
		&request.probabilities,
		&request.targets,
		&request.per_element_bce,
	]
}

fn output_tensors(request: &BinaryClassificationMetricRequest) -> Vec<&Tensor> {
	let mut tensors = vec![
		&request.mean_bce,
		&request.auroc,
		&request.auprc,
		&request.brier_score,
		&request.expected_calibration_error,
	];
	tensors.extend(request.recall_at.iter().map(|recall| &recall.output));
	tensors
}

fn require_dtype(name: &str, tensor: &Tensor, dtype: DType) -> OperationResult<()> {
	if tensor.dtype == dtype {
		Ok(())
	} else {
		Err(metric_error(
			OperationErrorKind::InvalidMaterializationRequest,
			format!("{name} must be {dtype:?}, received {:?}", tensor.dtype),
		))
	}
}

fn require_scalar_output(name: &str, tensor: &Tensor) -> OperationResult<()> {
	require_dtype(name, tensor, DType::F32)?;
	if tensor.shape.extents() == [1] {
		Ok(())
	} else {
		Err(metric_error(
			OperationErrorKind::UnsupportedConcreteShape,
			format!("{name} must have scalar shape [1]"),
		))
	}
}

fn scalar_shape() -> OperationResult<Shape> {
	Shape::new(vec![1]).map_err(|error| {
		metric_error(
			OperationErrorKind::GraphMaterializationFailed,
			error.to_string(),
		)
	})
}

fn sum_reduction(axis: usize, tree_lanes: u32) -> OperationResult<PrimitiveKind> {
	let axes = AxisSet::new(vec![axis]).map_err(|error| {
		metric_error(
			OperationErrorKind::GraphMaterializationFailed,
			error.to_string(),
		)
	})?;
	Ok(PrimitiveKind::Reduce(Reduce {
		operator: ReduceOperator::Sum,
		axes,
		keep_dimensions: false,
		result: ReduceResult::Value,
		tree_lanes,
	}))
}

fn inclusive_sum_scan(tree_lanes: u32) -> PrimitiveKind {
	PrimitiveKind::Scan(Scan {
		operator: ReduceOperator::Sum,
		axis: 0,
		mode: ScanMode::Inclusive,
		reverse: false,
		tree_lanes,
	})
}

fn gather(bounds: IndexBounds) -> PrimitiveKind { PrimitiveKind::Gather(Gather { axis: 0, bounds }) }

fn elementwise(program: recipe_core::ScalarProgram) -> PrimitiveKind {
	PrimitiveKind::Elementwise(Elementwise { program })
}

fn validated_inputs_program() -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder()?;
	let probability = scalar_input(&mut builder, DType::F32)?;
	let target = scalar_input(&mut builder, DType::F32)?;
	let loss = scalar_input(&mut builder, DType::F32)?;
	let zero = scalar_f32(&mut builder, 0.0)?;
	let one = scalar_f32(&mut builder, 1.0)?;

	let probability_finite = scalar_unary(&mut builder, ScalarOpcode::IsFinite, probability)?;
	let probability_low = scalar_binary(
		&mut builder,
		ScalarOpcode::GreaterThanOrEqual,
		probability,
		zero,
	)?;
	let probability_high = scalar_binary(
		&mut builder,
		ScalarOpcode::LessThanOrEqual,
		probability,
		one,
	)?;
	let probability_range = scalar_binary(
		&mut builder,
		ScalarOpcode::BitAnd,
		probability_low,
		probability_high,
	)?;
	let probability_valid = scalar_binary(
		&mut builder,
		ScalarOpcode::BitAnd,
		probability_finite,
		probability_range,
	)?;
	let probability_required = scalar_unary(&mut builder, ScalarOpcode::Require, probability_valid)?;
	let probability = scalar_ternary(
		&mut builder,
		ScalarOpcode::Select,
		probability_required,
		probability,
		probability,
	)?;

	let target_finite = scalar_unary(&mut builder, ScalarOpcode::IsFinite, target)?;
	let target_zero = scalar_binary(&mut builder, ScalarOpcode::Equal, target, zero)?;
	let target_one = scalar_binary(&mut builder, ScalarOpcode::Equal, target, one)?;
	let target_binary = scalar_binary(&mut builder, ScalarOpcode::BitOr, target_zero, target_one)?;
	let target_valid = scalar_binary(
		&mut builder,
		ScalarOpcode::BitAnd,
		target_finite,
		target_binary,
	)?;
	let target_required = scalar_unary(&mut builder, ScalarOpcode::Require, target_valid)?;
	let target = scalar_ternary(
		&mut builder,
		ScalarOpcode::Select,
		target_required,
		target,
		target,
	)?;

	let loss_finite = scalar_unary(&mut builder, ScalarOpcode::IsFinite, loss)?;
	let loss_nonnegative = scalar_binary(&mut builder, ScalarOpcode::GreaterThanOrEqual, loss, zero)?;
	let loss_valid = scalar_binary(
		&mut builder,
		ScalarOpcode::BitAnd,
		loss_finite,
		loss_nonnegative,
	)?;
	let loss_required = scalar_unary(&mut builder, ScalarOpcode::Require, loss_valid)?;
	let loss = scalar_ternary(
		&mut builder,
		ScalarOpcode::Select,
		loss_required,
		loss,
		loss,
	)?;
	scalar_finish(builder, &[probability, target, loss])
}

fn divide_by_constant_program(divisor: f32) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder()?;
	let value = scalar_input(&mut builder, DType::F32)?;
	let divisor = scalar_f32(&mut builder, divisor)?;
	let result = scalar_binary(&mut builder, ScalarOpcode::Divide, value, divisor)?;
	scalar_finish(builder, &[result])
}

fn brier_program() -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder()?;
	let probability = scalar_input(&mut builder, DType::F32)?;
	let target = scalar_input(&mut builder, DType::F32)?;
	let residual = scalar_binary(&mut builder, ScalarOpcode::Subtract, probability, target)?;
	let squared = scalar_binary(&mut builder, ScalarOpcode::Multiply, residual, residual)?;
	scalar_finish(builder, &[squared])
}

fn group_start_program() -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder()?;
	let current = scalar_input(&mut builder, DType::F32)?;
	let previous = scalar_input(&mut builder, DType::F32)?;
	let position = scalar_input(&mut builder, DType::I32)?;
	let zero = scalar_i32(&mut builder, 0)?;
	let first = scalar_binary(&mut builder, ScalarOpcode::Equal, position, zero)?;
	let changed = scalar_binary(&mut builder, ScalarOpcode::NotEqual, current, previous)?;
	let starts = scalar_binary(&mut builder, ScalarOpcode::BitOr, first, changed)?;
	scalar_finish(builder, &[starts])
}

fn subtract_one_program() -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder()?;
	let value = scalar_input(&mut builder, DType::I32)?;
	let one = scalar_i32(&mut builder, 1)?;
	let result = scalar_binary(&mut builder, ScalarOpcode::Subtract, value, one)?;
	scalar_finish(builder, &[result])
}

fn ranking_contributions_program(population: f32) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder()?;
	let group_count = scalar_input(&mut builder, DType::I32)?;
	let group_positive = scalar_input(&mut builder, DType::F32)?;
	let cumulative_count = scalar_input(&mut builder, DType::I32)?;
	let cumulative_positive = scalar_input(&mut builder, DType::F32)?;
	let total_positive = scalar_input(&mut builder, DType::F32)?;
	let group_count = scalar_unary(&mut builder, ScalarOpcode::ConvertI32ToF32, group_count)?;
	let cumulative_count = scalar_unary(
		&mut builder,
		ScalarOpcode::ConvertI32ToF32,
		cumulative_count,
	)?;
	let population = scalar_f32(&mut builder, population)?;
	let half = scalar_f32(&mut builder, 0.5)?;

	let group_negative = scalar_binary(
		&mut builder,
		ScalarOpcode::Subtract,
		group_count,
		group_positive,
	)?;
	let cumulative_negative = scalar_binary(
		&mut builder,
		ScalarOpcode::Subtract,
		cumulative_count,
		cumulative_positive,
	)?;
	let total_negative = scalar_binary(
		&mut builder,
		ScalarOpcode::Subtract,
		population,
		total_positive,
	)?;
	let negatives_below = scalar_binary(
		&mut builder,
		ScalarOpcode::Subtract,
		total_negative,
		cumulative_negative,
	)?;
	let half_ties = scalar_binary(&mut builder, ScalarOpcode::Multiply, half, group_negative)?;
	let rank_credit = scalar_binary(&mut builder, ScalarOpcode::Add, negatives_below, half_ties)?;
	let auroc = scalar_binary(
		&mut builder,
		ScalarOpcode::Multiply,
		group_positive,
		rank_credit,
	)?;

	let precision = scalar_binary(
		&mut builder,
		ScalarOpcode::Divide,
		cumulative_positive,
		cumulative_count,
	)?;
	let auprc = scalar_binary(
		&mut builder,
		ScalarOpcode::Multiply,
		group_positive,
		precision,
	)?;
	scalar_finish(builder, &[auroc, auprc])
}

fn normalize_ranking_program(population: f32) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder()?;
	let positives = scalar_input(&mut builder, DType::F32)?;
	let auroc_numerator = scalar_input(&mut builder, DType::F32)?;
	let auprc_numerator = scalar_input(&mut builder, DType::F32)?;
	let zero = scalar_f32(&mut builder, 0.0)?;
	let population = scalar_f32(&mut builder, population)?;
	let negatives = scalar_binary(&mut builder, ScalarOpcode::Subtract, population, positives)?;
	let has_positives = scalar_binary(&mut builder, ScalarOpcode::GreaterThan, positives, zero)?;
	let has_negatives = scalar_binary(&mut builder, ScalarOpcode::GreaterThan, negatives, zero)?;
	let valid_auc_population = scalar_binary(
		&mut builder,
		ScalarOpcode::BitAnd,
		has_positives,
		has_negatives,
	)?;
	let auc_required = scalar_unary(&mut builder, ScalarOpcode::Require, valid_auc_population)?;
	let positives_required = scalar_unary(&mut builder, ScalarOpcode::Require, has_positives)?;
	let auc_denominator = scalar_binary(&mut builder, ScalarOpcode::Multiply, positives, negatives)?;
	let auc_denominator = scalar_ternary(
		&mut builder,
		ScalarOpcode::Select,
		auc_required,
		auc_denominator,
		auc_denominator,
	)?;
	let positives = scalar_ternary(
		&mut builder,
		ScalarOpcode::Select,
		positives_required,
		positives,
		positives,
	)?;
	let auroc = scalar_binary(
		&mut builder,
		ScalarOpcode::Divide,
		auroc_numerator,
		auc_denominator,
	)?;
	let auprc = scalar_binary(
		&mut builder,
		ScalarOpcode::Divide,
		auprc_numerator,
		positives,
	)?;
	scalar_finish(builder, &[auroc, auprc])
}

fn recall_hits_program(threshold: f32) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder()?;
	let probability = scalar_input(&mut builder, DType::F32)?;
	let target = scalar_input(&mut builder, DType::F32)?;
	let threshold = scalar_f32(&mut builder, threshold)?;
	let predicted = scalar_binary(
		&mut builder,
		ScalarOpcode::GreaterThanOrEqual,
		probability,
		threshold,
	)?;
	let predicted = scalar_unary(&mut builder, ScalarOpcode::ConvertI32ToF32, predicted)?;
	let hit = scalar_binary(&mut builder, ScalarOpcode::Multiply, target, predicted)?;
	scalar_finish(builder, &[hit])
}

fn normalize_positive_count_program() -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder()?;
	let numerator = scalar_input(&mut builder, DType::F32)?;
	let positives = scalar_input(&mut builder, DType::F32)?;
	let zero = scalar_f32(&mut builder, 0.0)?;
	let has_positives = scalar_binary(&mut builder, ScalarOpcode::GreaterThan, positives, zero)?;
	let required = scalar_unary(&mut builder, ScalarOpcode::Require, has_positives)?;
	let positives = scalar_ternary(
		&mut builder,
		ScalarOpcode::Select,
		required,
		positives,
		positives,
	)?;
	let result = scalar_binary(&mut builder, ScalarOpcode::Divide, numerator, positives)?;
	scalar_finish(builder, &[result])
}

fn calibration_bin_program(lower: f32, upper: f32, include_upper: bool) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder()?;
	let probability = scalar_input(&mut builder, DType::F32)?;
	let target = scalar_input(&mut builder, DType::F32)?;
	let lower = scalar_f32(&mut builder, lower)?;
	let upper = scalar_f32(&mut builder, upper)?;
	let above_lower = scalar_binary(
		&mut builder,
		ScalarOpcode::GreaterThanOrEqual,
		probability,
		lower,
	)?;
	let below_upper = scalar_binary(
		&mut builder,
		if include_upper {
			ScalarOpcode::LessThanOrEqual
		} else {
			ScalarOpcode::LessThan
		},
		probability,
		upper,
	)?;
	let member = scalar_binary(&mut builder, ScalarOpcode::BitAnd, above_lower, below_upper)?;
	let member = scalar_unary(&mut builder, ScalarOpcode::ConvertI32ToF32, member)?;
	let confidence = scalar_binary(&mut builder, ScalarOpcode::Multiply, probability, member)?;
	let target = scalar_binary(&mut builder, ScalarOpcode::Multiply, target, member)?;
	scalar_finish(builder, &[confidence, target])
}

fn calibration_contribution_program(population: f32) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder()?;
	let confidence_sum = scalar_input(&mut builder, DType::F32)?;
	let target_sum = scalar_input(&mut builder, DType::F32)?;
	let population = scalar_f32(&mut builder, population)?;
	let difference = scalar_binary(
		&mut builder,
		ScalarOpcode::Subtract,
		confidence_sum,
		target_sum,
	)?;
	let difference = scalar_unary(&mut builder, ScalarOpcode::Absolute, difference)?;
	let result = scalar_binary(&mut builder, ScalarOpcode::Divide, difference, population)?;
	scalar_finish(builder, &[result])
}

fn fixed_sum_program(inputs: usize) -> OperationResult<recipe_core::ScalarProgram> {
	if inputs == 0 {
		return Err(metric_error(
			OperationErrorKind::InvalidMaterializationRequest,
			"fixed scalar sum requires at least one input",
		));
	}
	let mut builder = scalar_builder()?;
	let mut level = (0..inputs)
		.map(|_| scalar_input(&mut builder, DType::F32))
		.collect::<OperationResult<Vec<_>>>()?;
	while level.len() > 1 {
		let mut next = Vec::with_capacity(level.len().div_ceil(2));
		for pair in level.chunks(2) {
			next.push(match pair {
				[left, right] => scalar_binary(&mut builder, ScalarOpcode::Add, *left, *right)?,
				[left] => *left,
				_ => unreachable!("chunks of two are nonempty"),
			});
		}
		level = next;
	}
	scalar_finish(builder, &[level[0]])
}

fn scalar_builder() -> OperationResult<ScalarProgramBuilder> {
	ScalarProgramBuilder::new().map_err(|error| {
		metric_error(
			OperationErrorKind::GraphMaterializationFailed,
			error.to_string(),
		)
	})
}

fn scalar_input(builder: &mut ScalarProgramBuilder, dtype: DType) -> OperationResult<ScalarExpression> {
	builder.input(dtype).map_err(|error| {
		metric_error(
			OperationErrorKind::GraphMaterializationFailed,
			error.to_string(),
		)
	})
}

fn scalar_f32(builder: &mut ScalarProgramBuilder, value: f32) -> OperationResult<ScalarExpression> {
	builder.f32(value).map_err(|error| {
		metric_error(
			OperationErrorKind::GraphMaterializationFailed,
			error.to_string(),
		)
	})
}

fn scalar_i32(builder: &mut ScalarProgramBuilder, value: i32) -> OperationResult<ScalarExpression> {
	builder.i32(value).map_err(|error| {
		metric_error(
			OperationErrorKind::GraphMaterializationFailed,
			error.to_string(),
		)
	})
}

fn scalar_unary(
	builder: &mut ScalarProgramBuilder,
	opcode: ScalarOpcode,
	value: ScalarExpression,
) -> OperationResult<ScalarExpression> {
	builder.unary(opcode, value).map_err(|error| {
		metric_error(
			OperationErrorKind::GraphMaterializationFailed,
			error.to_string(),
		)
	})
}

fn scalar_binary(
	builder: &mut ScalarProgramBuilder,
	opcode: ScalarOpcode,
	left: ScalarExpression,
	right: ScalarExpression,
) -> OperationResult<ScalarExpression> {
	builder.binary(opcode, left, right).map_err(|error| {
		metric_error(
			OperationErrorKind::GraphMaterializationFailed,
			error.to_string(),
		)
	})
}

fn scalar_ternary(
	builder: &mut ScalarProgramBuilder,
	opcode: ScalarOpcode,
	first: ScalarExpression,
	second: ScalarExpression,
	third: ScalarExpression,
) -> OperationResult<ScalarExpression> {
	builder
		.ternary(opcode, first, second, third)
		.map_err(|error| {
			metric_error(
				OperationErrorKind::GraphMaterializationFailed,
				error.to_string(),
			)
		})
}

fn scalar_finish(
	builder: ScalarProgramBuilder,
	outputs: &[ScalarExpression],
) -> OperationResult<recipe_core::ScalarProgram> {
	builder.finish(outputs).map_err(|error| {
		metric_error(
			OperationErrorKind::GraphMaterializationFailed,
			error.to_string(),
		)
	})
}

fn forbidden_aliases(inputs: usize, outputs: usize) -> Vec<PrimitiveAliasRule> {
	(0..inputs)
		.flat_map(|input| {
			(0..outputs).map(move |output| {
				PrimitiveAliasRule {
					input,
					output,
					permission: AliasPermission::Forbidden,
				}
			})
		})
		.collect()
}

fn checked_mul(left: u64, right: u64, context: &str) -> OperationResult<u64> {
	left.checked_mul(right).ok_or_else(|| {
		metric_error(
			OperationErrorKind::WorkspaceArithmeticOverflow,
			format!("{context} overflows u64"),
		)
	})
}

fn checked_add(left: u64, right: u64, context: &str) -> OperationResult<u64> {
	left.checked_add(right).ok_or_else(|| {
		metric_error(
			OperationErrorKind::WorkspaceArithmeticOverflow,
			format!("{context} overflows u64"),
		)
	})
}

fn metric_error(kind: OperationErrorKind, detail: impl Into<String>) -> OperationError {
	OperationError::new(kind, detail)
}
