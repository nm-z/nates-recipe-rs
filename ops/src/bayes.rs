use std::collections::{BTreeMap, BTreeSet};

use recipe_core::{AliasPermission, ByteCount, DType, KernelTemplateId, ScalarOpcode, ValueId};
use recipe_language::{
	AtomicOrdering, AxisSet, CalculationGraph, CalculationNode, Elementwise, Gather, Histogram, IndexBounds,
	IndexMap, PrimitiveAliasRule, PrimitiveKernel, PrimitiveKind, Reduce, ReduceOperator, ReduceResult,
	ScalarExpression, ScalarProgramBuilder, Shape, Tensor,
};

use crate::{IdentityNamespace, OperationError, OperationErrorKind, OperationResult};

const INTERMEDIATE_VALUES: u64 = 10;
const KERNELS: u64 = 11;
const MAXIMUM_TREE_LANES: u32 = 1_024;

/// One observed categorical conditional distribution evaluated from saved
/// reference observations.
///
/// Parent codes are packed in declaration order using `parent_multipliers`.
/// `parent_cardinalities` includes each parent's reserved unseen route, so a
/// novel inference label selects an unobserved configuration and therefore a
/// uniform Laplace posterior. Child codes never include a reserved class.
#[derive(Clone, Debug, PartialEq)]
pub struct CategoricalBayesInferenceRequest {
	pub reference_parent_codes: Tensor,
	pub reference_child_codes: Tensor,
	pub query_parent_codes: Tensor,
	pub parent_multipliers: Tensor,
	pub parent_cardinalities: Tensor,
	pub probabilities: Tensor,
	pub reference_rows: u64,
	pub query_rows: u64,
	pub parent_count: u64,
	pub parent_configurations: u64,
	pub child_classes: u64,
	pub smoothing: f32,
	pub tree_lanes: u32,
	pub identity_namespace: IdentityNamespace,
	pub workspace_limit: ByteCount,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CategoricalBayesInferenceRequirements {
	pub intermediate_values: u64,
	pub kernels: u64,
	pub workspace_bytes: ByteCount,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CategoricalBayesInferenceMaterialization {
	pub graph: CalculationGraph,
	pub intermediate_values: Vec<ValueId>,
	pub kernels: Vec<KernelTemplateId>,
	pub workspace_bytes: ByteCount,
	pub identity_namespace: IdentityNamespace,
}

pub fn categorical_bayes_inference_requirements(
	reference_rows: u64,
	query_rows: u64,
	parent_count: u64,
	parent_configurations: u64,
	child_classes: u64,
) -> OperationResult<CategoricalBayesInferenceRequirements> {
	for (name, value) in [
		("reference row count", reference_rows),
		("query row count", query_rows),
		("parent count", parent_count),
		("parent configuration count", parent_configurations),
		("child class count", child_classes),
	] {
		if value == 0 {
			return Err(bayes_error(
				OperationErrorKind::UnsupportedConcreteShape,
				format!("categorical Bayes {name} must be nonzero"),
			));
		}
	}
	let bins = checked_product(
		&[parent_configurations, child_classes],
		"categorical Bayes histogram bins",
	)?;
	if bins > i32::MAX as u64 || bins > u32::MAX as u64 {
		return Err(bayes_error(
			OperationErrorKind::UnsupportedConcreteShape,
			"categorical Bayes joint state space exceeds the checked histogram index domain",
		));
	}
	if parent_configurations > i32::MAX as u64 || parent_count > i32::MAX as u64 || child_classes > i32::MAX as u64 {
		return Err(bayes_error(
			OperationErrorKind::UnsupportedConcreteShape,
			"categorical Bayes dimensions exceed the checked int32 domain",
		));
	}
	let reference_parent_elements = checked_product(
		&[reference_rows, parent_count],
		"categorical Bayes reference parent elements",
	)?;
	let query_parent_elements = checked_product(
		&[query_rows, parent_count],
		"categorical Bayes query parent elements",
	)?;
	let query_output_elements = checked_product(
		&[query_rows, child_classes],
		"categorical Bayes query output elements",
	)?;
	let workspace_elements = checked_sum(
		&[
			reference_parent_elements,
			reference_rows,
			reference_rows,
			bins,
			query_parent_elements,
			query_rows,
			child_classes,
			query_output_elements,
			query_output_elements,
			query_rows,
		],
		"categorical Bayes workspace elements",
	)?;
	Ok(CategoricalBayesInferenceRequirements {
		intermediate_values: INTERMEDIATE_VALUES,
		kernels: KERNELS,
		workspace_bytes: ByteCount::new(checked_product(
			&[workspace_elements, 4],
			"categorical Bayes workspace bytes",
		)?),
	})
}

pub fn materialize_categorical_bayes_inference(
	request: &CategoricalBayesInferenceRequest,
) -> OperationResult<CategoricalBayesInferenceMaterialization> {
	let requirements = validate_request(request)?;
	validate_resources(request, requirements)?;
	let mut emitter = BayesEmitter::new(request, requirements)?;

	let reference_contributions = emitter.intermediate(
		DType::I32,
		shape(&[request.reference_rows, request.parent_count])?,
	)?;
	emitter.elementwise(
		vec![
			request.reference_parent_codes.id,
			request.parent_multipliers.id,
			request.parent_cardinalities.id,
		],
		vec![reference_contributions],
		configuration_contribution_program()?,
	)?;
	let reference_configurations = emitter.intermediate(DType::I32, shape(&[request.reference_rows])?)?;
	emitter.reduce_sum(
		reference_contributions,
		reference_configurations,
		1,
		false,
		request.tree_lanes,
	)?;
	let reference_joint_bins = emitter.intermediate(DType::I32, shape(&[request.reference_rows])?)?;
	emitter.elementwise(
		vec![reference_configurations, request.reference_child_codes.id],
		vec![reference_joint_bins],
		joint_bin_program(request.child_classes)?,
	)?;
	let joint_bins = checked_product(
		&[request.parent_configurations, request.child_classes],
		"categorical Bayes histogram bins",
	)?;
	let counts = emitter.intermediate(DType::I32, shape(&[joint_bins])?)?;
	emitter.emit(
		vec![reference_joint_bins],
		vec![counts],
		PrimitiveKind::Histogram(Histogram {
			bins: u32::try_from(joint_bins).map_err(|error| {
				bayes_error(
					OperationErrorKind::UnsupportedConcreteShape,
					format!("categorical Bayes histogram bin count does not fit u32: {error}"),
				)
			})?,
			weighted: false,
			ordering: AtomicOrdering::Relaxed,
		}),
	)?;

	let query_contributions = emitter.intermediate(
		DType::I32,
		shape(&[request.query_rows, request.parent_count])?,
	)?;
	emitter.elementwise(
		vec![
			request.query_parent_codes.id,
			request.parent_multipliers.id,
			request.parent_cardinalities.id,
		],
		vec![query_contributions],
		configuration_contribution_program()?,
	)?;
	let query_configurations = emitter.intermediate(DType::I32, shape(&[request.query_rows, 1])?)?;
	emitter.reduce_sum(
		query_contributions,
		query_configurations,
		1,
		true,
		request.tree_lanes,
	)?;
	let class_offsets = emitter.intermediate(DType::I32, shape(&[1, request.child_classes])?)?;
	emitter.index_map(class_offsets, 0, 1)?;
	let query_bins = emitter.intermediate(
		DType::I32,
		shape(&[request.query_rows, request.child_classes])?,
	)?;
	emitter.elementwise(
		vec![query_configurations, class_offsets],
		vec![query_bins],
		query_bin_program(request.child_classes)?,
	)?;
	let selected_counts = emitter.intermediate(
		DType::I32,
		shape(&[request.query_rows, request.child_classes])?,
	)?;
	emitter.gather(counts, query_bins, selected_counts)?;
	let totals = emitter.intermediate(DType::I32, shape(&[request.query_rows, 1])?)?;
	emitter.reduce_sum(selected_counts, totals, 1, true, request.tree_lanes)?;
	emitter.elementwise(
		vec![selected_counts, totals],
		vec![request.probabilities.id],
		posterior_probability_program(request.child_classes, request.smoothing)?,
	)?;
	emitter.finish()
}

pub fn append_categorical_bayes_inference(
	graph: &mut CalculationGraph,
	request: &CategoricalBayesInferenceRequest,
) -> OperationResult<CategoricalBayesInferenceMaterialization> {
	validate_boundary_storage(graph, request)?;
	let materialized = materialize_categorical_bayes_inference(request)?;
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
		return Err(bayes_error(
			OperationErrorKind::IdentityNamespaceOverlap,
			format!("categorical Bayes intermediate {value} already exists in caller graph"),
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
		return Err(bayes_error(
			OperationErrorKind::IdentityNamespaceOverlap,
			format!("categorical Bayes kernel {kernel} already exists in caller graph"),
		));
	}
	if graph
		.nodes
		.iter()
		.any(|node| node.kernel.outputs.contains(&request.probabilities.id))
	{
		return Err(bayes_error(
			OperationErrorKind::GraphMaterializationFailed,
			"caller graph already produces categorical Bayes probabilities",
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

fn validate_request(
	request: &CategoricalBayesInferenceRequest,
) -> OperationResult<CategoricalBayesInferenceRequirements> {
	for tensor in [
		&request.reference_parent_codes,
		&request.reference_child_codes,
		&request.query_parent_codes,
		&request.parent_multipliers,
		&request.parent_cardinalities,
		&request.probabilities,
	] {
		tensor.validate().map_err(graph_error)?;
	}
	let requirements = categorical_bayes_inference_requirements(
		request.reference_rows,
		request.query_rows,
		request.parent_count,
		request.parent_configurations,
		request.child_classes,
	)?;
	for (name, tensor, dtype, extents) in [
		(
			"reference parent codes",
			&request.reference_parent_codes,
			DType::I32,
			vec![request.reference_rows, request.parent_count],
		),
		(
			"reference child codes",
			&request.reference_child_codes,
			DType::I32,
			vec![request.reference_rows],
		),
		(
			"query parent codes",
			&request.query_parent_codes,
			DType::I32,
			vec![request.query_rows, request.parent_count],
		),
		(
			"parent multipliers",
			&request.parent_multipliers,
			DType::I32,
			vec![request.parent_count],
		),
		(
			"parent cardinalities",
			&request.parent_cardinalities,
			DType::I32,
			vec![request.parent_count],
		),
		(
			"posterior probabilities",
			&request.probabilities,
			DType::F32,
			vec![request.query_rows, request.child_classes],
		),
	] {
		if tensor.dtype != dtype || tensor.shape.extents() != extents {
			return Err(bayes_error(
				OperationErrorKind::UnsupportedConcreteShape,
				format!("categorical Bayes {name} must be {dtype:?} {extents:?}"),
			));
		}
	}
	if !request.smoothing.is_finite() || request.smoothing <= 0.0 {
		return Err(bayes_error(
			OperationErrorKind::InvalidMaterializationRequest,
			"categorical Bayes smoothing must be finite and positive",
		));
	}
	if request.tree_lanes == 0 || request.tree_lanes > MAXIMUM_TREE_LANES || !request.tree_lanes.is_power_of_two() {
		return Err(bayes_error(
			OperationErrorKind::InvalidMaterializationRequest,
			format!("categorical Bayes reduction lanes must be a power of two in 1..={MAXIMUM_TREE_LANES}"),
		));
	}
	Ok(requirements)
}

fn validate_boundary_storage(
	graph: &CalculationGraph,
	request: &CategoricalBayesInferenceRequest,
) -> OperationResult<()> {
	let mut ids = BTreeSet::new();
	for tensor in &graph.tensors {
		if !ids.insert(tensor.id) {
			return Err(bayes_error(
				OperationErrorKind::GraphMaterializationFailed,
				format!("caller graph repeats tensor {}", tensor.id),
			));
		}
	}
	for declared in [
		&request.reference_parent_codes,
		&request.reference_child_codes,
		&request.query_parent_codes,
		&request.parent_multipliers,
		&request.parent_cardinalities,
		&request.probabilities,
	] {
		let existing = graph
			.tensors
			.iter()
			.find(|tensor| tensor.id == declared.id)
			.ok_or_else(|| {
				bayes_error(
					OperationErrorKind::InvalidMaterializationRequest,
					format!(
						"caller graph does not declare categorical Bayes boundary tensor {}",
						declared.id
					),
				)
			})?;
		if !same_storage_contract(existing, declared) {
			return Err(bayes_error(
				OperationErrorKind::InvalidMaterializationRequest,
				format!(
					"caller graph tensor {} differs from the categorical Bayes storage contract",
					declared.id
				),
			));
		}
	}
	Ok(())
}

fn validate_resources(
	request: &CategoricalBayesInferenceRequest,
	requirements: CategoricalBayesInferenceRequirements,
) -> OperationResult<()> {
	if requirements.intermediate_values > request.identity_namespace.value_capacity() {
		return Err(bayes_error(
			OperationErrorKind::IdentityNamespaceExhausted,
			"categorical Bayes intermediate identity range is too small",
		));
	}
	if requirements.kernels > request.identity_namespace.kernel_capacity() {
		return Err(bayes_error(
			OperationErrorKind::IdentityNamespaceExhausted,
			"categorical Bayes kernel identity range is too small",
		));
	}
	if requirements.workspace_bytes.get() > request.workspace_limit.get() {
		return Err(bayes_error(
			OperationErrorKind::WorkspaceLimitExceeded,
			format!(
				"categorical Bayes needs {} workspace bytes, limit is {}",
				requirements.workspace_bytes.get(),
				request.workspace_limit.get()
			),
		));
	}
	Ok(())
}

struct BayesEmitter {
	tensors: Vec<Tensor>,
	nodes: Vec<CalculationNode>,
	intermediate_values: Vec<ValueId>,
	kernels: Vec<KernelTemplateId>,
	next_value: u64,
	value_end: u64,
	next_kernel: u64,
	kernel_end: u64,
	workspace_bytes: u64,
	requirements: CategoricalBayesInferenceRequirements,
	identity_namespace: IdentityNamespace,
}

impl BayesEmitter {
	fn new(
		request: &CategoricalBayesInferenceRequest,
		requirements: CategoricalBayesInferenceRequirements,
	) -> OperationResult<Self> {
		let first_value = request.identity_namespace.first_value().get();
		let value_end = first_value
			.checked_add(request.identity_namespace.value_capacity())
			.ok_or_else(|| {
				bayes_error(
					OperationErrorKind::IdentityNamespaceExhausted,
					"Bayes value range overflow",
				)
			})?;
		let first_kernel = request.identity_namespace.first_kernel().get();
		let kernel_end = first_kernel
			.checked_add(request.identity_namespace.kernel_capacity())
			.ok_or_else(|| {
				bayes_error(
					OperationErrorKind::IdentityNamespaceExhausted,
					"Bayes kernel range overflow",
				)
			})?;
		let mut boundaries = BTreeMap::<ValueId, Tensor>::new();
		for source in [
			&request.reference_parent_codes,
			&request.reference_child_codes,
			&request.query_parent_codes,
			&request.parent_multipliers,
			&request.parent_cardinalities,
		] {
			let mut tensor = source.clone();
			tensor.external_input = true;
			tensor.external_output = false;
			insert_boundary(&mut boundaries, tensor)?;
		}
		let mut probabilities = request.probabilities.clone();
		probabilities.external_input = false;
		probabilities.external_output = true;
		if boundaries.contains_key(&probabilities.id) {
			return Err(bayes_error(
				OperationErrorKind::InvalidMaterializationRequest,
				"categorical Bayes probabilities must not alias an input",
			));
		}
		insert_boundary(&mut boundaries, probabilities)?;
		if let Some(tensor) = boundaries
			.values()
			.find(|tensor| tensor.id.get() >= first_value && tensor.id.get() < value_end)
		{
			return Err(bayes_error(
				OperationErrorKind::IdentityNamespaceOverlap,
				format!(
					"boundary tensor {} overlaps categorical Bayes intermediates",
					tensor.id
				),
			));
		}
		Ok(Self {
			tensors: boundaries.into_values().collect(),
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
			return Err(bayes_error(
				OperationErrorKind::IdentityNamespaceExhausted,
				"categorical Bayes intermediate identity range is exhausted",
			));
		}
		let value = ValueId::new(self.next_value);
		self.next_value += 1;
		let tensor = Tensor::contiguous(value, dtype, shape, false, false).map_err(graph_error)?;
		self.workspace_bytes = self
			.workspace_bytes
			.checked_add(tensor.storage_bytes.get())
			.ok_or_else(|| {
				bayes_error(
					OperationErrorKind::WorkspaceArithmeticOverflow,
					"categorical Bayes workspace total overflowed u64",
				)
			})?;
		self.intermediate_values.push(value);
		self.tensors.push(tensor);
		Ok(value)
	}

	fn emit(&mut self, inputs: Vec<ValueId>, outputs: Vec<ValueId>, kind: PrimitiveKind) -> OperationResult<()> {
		if self.next_kernel == self.kernel_end {
			return Err(bayes_error(
				OperationErrorKind::IdentityNamespaceExhausted,
				"categorical Bayes kernel identity range is exhausted",
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

	fn elementwise(
		&mut self,
		inputs: Vec<ValueId>,
		outputs: Vec<ValueId>,
		program: recipe_core::ScalarProgram,
	) -> OperationResult<()> {
		self.emit(
			inputs,
			outputs,
			PrimitiveKind::Elementwise(Elementwise { program }),
		)
	}

	fn index_map(&mut self, output: ValueId, start: i32, element_step: i32) -> OperationResult<()> {
		self.emit(
			Vec::new(),
			vec![output],
			PrimitiveKind::IndexMap(IndexMap {
				start,
				element_step,
				iteration_step: 0,
				modulus: None,
			}),
		)
	}

	fn reduce_sum(
		&mut self,
		input: ValueId,
		output: ValueId,
		axis: usize,
		keep_dimensions: bool,
		tree_lanes: u32,
	) -> OperationResult<()> {
		self.emit(
			vec![input],
			vec![output],
			PrimitiveKind::Reduce(Reduce {
				operator: ReduceOperator::Sum,
				axes: AxisSet::new(vec![axis]).map_err(graph_error)?,
				keep_dimensions,
				result: ReduceResult::Value,
				tree_lanes,
			}),
		)
	}

	fn gather(&mut self, values: ValueId, indices: ValueId, output: ValueId) -> OperationResult<()> {
		self.emit(
			vec![values, indices],
			vec![output],
			PrimitiveKind::Gather(Gather {
				axis: 0,
				bounds: IndexBounds::Reject,
			}),
		)
	}

	fn finish(self) -> OperationResult<CategoricalBayesInferenceMaterialization> {
		if self.intermediate_values.len() as u64 != self.requirements.intermediate_values
			|| self.kernels.len() as u64 != self.requirements.kernels
			|| self.workspace_bytes != self.requirements.workspace_bytes.get()
		{
			return Err(bayes_error(
				OperationErrorKind::WorkspaceFormulaMismatch,
				format!(
					"categorical Bayes emitted {}/{}/{} values/kernels/bytes, expected {}/{}/{}",
					self.intermediate_values.len(),
					self.kernels.len(),
					self.workspace_bytes,
					self.requirements.intermediate_values,
					self.requirements.kernels,
					self.requirements.workspace_bytes.get()
				),
			));
		}
		let graph = CalculationGraph {
			tensors: self.tensors,
			nodes: self.nodes,
		};
		graph.validate().map_err(graph_error)?;
		Ok(CategoricalBayesInferenceMaterialization {
			graph,
			intermediate_values: self.intermediate_values,
			kernels: self.kernels,
			workspace_bytes: ByteCount::new(self.workspace_bytes),
			identity_namespace: self.identity_namespace,
		})
	}
}

fn configuration_contribution_program() -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new().map_err(graph_error)?;
	let code = builder.input(DType::I32).map_err(graph_error)?;
	let multiplier = builder.input(DType::I32).map_err(graph_error)?;
	let cardinality = builder.input(DType::I32).map_err(graph_error)?;
	require_i32_range(&mut builder, code, cardinality)?;
	let contribution = builder
		.binary(ScalarOpcode::Multiply, code, multiplier)
		.map_err(graph_error)?;
	builder.finish(&[contribution]).map_err(graph_error)
}

fn joint_bin_program(child_classes: u64) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new().map_err(graph_error)?;
	let configuration = builder.input(DType::I32).map_err(graph_error)?;
	let child = builder.input(DType::I32).map_err(graph_error)?;
	let classes = scalar_i32(&mut builder, child_classes, "child class count")?;
	require_i32_range(&mut builder, child, classes)?;
	let base = builder
		.binary(ScalarOpcode::Multiply, configuration, classes)
		.map_err(graph_error)?;
	let bin = builder
		.binary(ScalarOpcode::Add, base, child)
		.map_err(graph_error)?;
	builder.finish(&[bin]).map_err(graph_error)
}

fn query_bin_program(child_classes: u64) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new().map_err(graph_error)?;
	let configuration = builder.input(DType::I32).map_err(graph_error)?;
	let class = builder.input(DType::I32).map_err(graph_error)?;
	let classes = scalar_i32(&mut builder, child_classes, "child class count")?;
	require_i32_range(&mut builder, class, classes)?;
	let base = builder
		.binary(ScalarOpcode::Multiply, configuration, classes)
		.map_err(graph_error)?;
	let bin = builder
		.binary(ScalarOpcode::Add, base, class)
		.map_err(graph_error)?;
	builder.finish(&[bin]).map_err(graph_error)
}

fn posterior_probability_program(child_classes: u64, smoothing: f32) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new().map_err(graph_error)?;
	let count = builder.input(DType::I32).map_err(graph_error)?;
	let total = builder.input(DType::I32).map_err(graph_error)?;
	let zero = builder.i32(0).map_err(graph_error)?;
	for value in [count, total] {
		let valid = builder
			.binary(ScalarOpcode::GreaterThanOrEqual, value, zero)
			.map_err(graph_error)?;
		let _ = builder
			.unary(ScalarOpcode::Require, valid)
			.map_err(graph_error)?;
	}
	let count = builder
		.unary(ScalarOpcode::ConvertI32ToF32, count)
		.map_err(graph_error)?;
	let total = builder
		.unary(ScalarOpcode::ConvertI32ToF32, total)
		.map_err(graph_error)?;
	let smoothing_value = builder.f32(smoothing).map_err(graph_error)?;
	let class_count = builder.f32(child_classes as f32).map_err(graph_error)?;
	let numerator = builder
		.binary(ScalarOpcode::Add, count, smoothing_value)
		.map_err(graph_error)?;
	let prior_mass = builder
		.binary(ScalarOpcode::Multiply, smoothing_value, class_count)
		.map_err(graph_error)?;
	let denominator = builder
		.binary(ScalarOpcode::Add, total, prior_mass)
		.map_err(graph_error)?;
	let probability = builder
		.binary(ScalarOpcode::Divide, numerator, denominator)
		.map_err(graph_error)?;
	builder.finish(&[probability]).map_err(graph_error)
}

fn require_i32_range(
	builder: &mut ScalarProgramBuilder,
	value: ScalarExpression,
	upper_exclusive: ScalarExpression,
) -> OperationResult<()> {
	let zero = builder.i32(0).map_err(graph_error)?;
	let lower = builder
		.binary(ScalarOpcode::GreaterThanOrEqual, value, zero)
		.map_err(graph_error)?;
	let upper = builder
		.binary(ScalarOpcode::LessThan, value, upper_exclusive)
		.map_err(graph_error)?;
	let valid = builder
		.binary(ScalarOpcode::BitAnd, lower, upper)
		.map_err(graph_error)?;
	let _ = builder
		.unary(ScalarOpcode::Require, valid)
		.map_err(graph_error)?;
	Ok(())
}

fn scalar_i32(builder: &mut ScalarProgramBuilder, value: u64, role: &str) -> OperationResult<ScalarExpression> {
	let value = i32::try_from(value).map_err(|error| {
		bayes_error(
			OperationErrorKind::UnsupportedConcreteShape,
			format!("categorical Bayes {role} does not fit i32: {error}"),
		)
	})?;
	builder.i32(value).map_err(graph_error)
}

fn same_storage_contract(left: &Tensor, right: &Tensor) -> bool {
	left.id == right.id
		&& left.dtype == right.dtype
		&& left.shape == right.shape
		&& left.layout == right.layout
		&& left.storage_bytes == right.storage_bytes
}

fn insert_boundary(boundaries: &mut BTreeMap<ValueId, Tensor>, tensor: Tensor) -> OperationResult<()> {
	match boundaries.get(&tensor.id) {
		Some(existing) if same_storage_contract(existing, &tensor) => Ok(()),
		Some(_) => {
			Err(bayes_error(
				OperationErrorKind::InvalidMaterializationRequest,
				format!(
					"categorical Bayes boundary tensor {} has conflicting contracts",
					tensor.id
				),
			))
		}
		None => {
			boundaries.insert(tensor.id, tensor);
			Ok(())
		}
	}
}

fn forbidden_aliases(input_count: usize, output_count: usize) -> Vec<PrimitiveAliasRule> {
	(0..input_count)
		.flat_map(|input| {
			(0..output_count).map(move |output| {
				PrimitiveAliasRule {
					input,
					output,
					permission: AliasPermission::Forbidden,
				}
			})
		})
		.collect()
}

fn shape(extents: &[u64]) -> OperationResult<Shape> { Shape::new(extents.to_vec()).map_err(graph_error) }

fn checked_product(values: &[u64], role: &str) -> OperationResult<u64> {
	values.iter().copied().try_fold(1_u64, |product, value| {
		product.checked_mul(value).ok_or_else(|| {
			bayes_error(
				OperationErrorKind::WorkspaceArithmeticOverflow,
				format!("{role} overflowed u64"),
			)
		})
	})
}

fn checked_sum(values: &[u64], role: &str) -> OperationResult<u64> {
	values.iter().copied().try_fold(0_u64, |sum, value| {
		sum.checked_add(value).ok_or_else(|| {
			bayes_error(
				OperationErrorKind::WorkspaceArithmeticOverflow,
				format!("{role} overflowed u64"),
			)
		})
	})
}

fn graph_error(error: impl ToString) -> OperationError {
	bayes_error(
		OperationErrorKind::GraphMaterializationFailed,
		error.to_string(),
	)
}

fn bayes_error(kind: OperationErrorKind, detail: impl Into<String>) -> OperationError {
	OperationError {
		operation: None,
		kind,
		detail: detail.into(),
	}
}
