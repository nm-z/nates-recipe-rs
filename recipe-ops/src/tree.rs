use std::collections::{BTreeMap, BTreeSet};

use recipe_core::{AliasPermission, ByteCount, DType, KernelTemplateId, ScalarOpcode, ValueId};
use recipe_language::{
	AxisSet, CalculationGraph, CalculationNode, Elementwise, Gather, IndexBounds, IndexMap, PrimitiveAliasRule,
	PrimitiveKernel, PrimitiveKind, Reduce, ReduceOperator, ReduceResult, ScalarExpression, ScalarProgramBuilder,
	Shape, Tensor,
};

use crate::{IdentityNamespace, OperationError, OperationErrorKind, OperationResult};

const MAXIMUM_TREE_LANES: u32 = 1_024;
const FIXED_INTERMEDIATES: u64 = 7;
const INTERMEDIATES_PER_DEPTH: u64 = 6;
const FIXED_KERNELS: u64 = 8;
const KERNELS_PER_DEPTH: u64 = 6;

/// Statically shaped inference for a saved ensemble of complete binary trees.
///
/// `features` is flattened row-major `[rows, feature_width]`. Split arrays use
/// tree-major heap order. Leaf values use tree-major
/// `[trees, 2^depth, outputs]` order. Exact threshold ties go left. Tree
/// outputs are summed in fixed tree order and multiplied by `scale`, so the
/// same traversal supports boosted sums and forest means.
#[derive(Clone, Debug, PartialEq)]
pub struct TreeEnsembleInferenceRequest {
	pub features: Tensor,
	pub split_features: Tensor,
	pub split_thresholds: Tensor,
	pub leaf_values: Tensor,
	pub predictions: Tensor,
	pub rows: u64,
	pub feature_width: u64,
	pub trees: u64,
	pub depth: u32,
	pub outputs: u64,
	pub scale: f32,
	pub tree_lanes: u32,
	pub identity_namespace: IdentityNamespace,
	pub workspace_limit: ByteCount,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TreeEnsembleInferenceRequirements {
	pub internal_nodes_per_tree: u64,
	pub leaves_per_tree: u64,
	pub intermediate_values: u64,
	pub kernels: u64,
	pub workspace_bytes: ByteCount,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeEnsembleInferenceMaterialization {
	pub graph: CalculationGraph,
	pub intermediate_values: Vec<ValueId>,
	pub kernels: Vec<KernelTemplateId>,
	pub internal_nodes_per_tree: u64,
	pub leaves_per_tree: u64,
	pub workspace_bytes: ByteCount,
	pub identity_namespace: IdentityNamespace,
}

pub fn tree_ensemble_inference_requirements(
	rows: u64,
	feature_width: u64,
	trees: u64,
	depth: u32,
	outputs: u64,
) -> OperationResult<TreeEnsembleInferenceRequirements> {
	for (name, value) in [
		("row count", rows),
		("feature width", feature_width),
		("tree count", trees),
		("output width", outputs),
	] {
		if value == 0 {
			return Err(tree_error(
				OperationErrorKind::UnsupportedConcreteShape,
				format!("tree ensemble {name} must be nonzero"),
			));
		}
	}
	if depth == 0 || depth >= 31 {
		return Err(tree_error(
			OperationErrorKind::UnsupportedConcreteShape,
			"tree depth must be in 1..=30",
		));
	}
	let leaves = 1_u64.checked_shl(depth).ok_or_else(|| {
		tree_error(
			OperationErrorKind::WorkspaceArithmeticOverflow,
			"tree leaf count overflowed u64",
		)
	})?;
	let internal_nodes = leaves - 1;
	let feature_elements = checked_product(&[rows, feature_width], "tree feature elements")?;
	let split_elements = checked_product(&[trees, internal_nodes], "tree split elements")?;
	let leaf_elements = checked_product(&[trees, leaves, outputs], "tree leaf-value elements")?;
	let pair_elements = checked_product(&[rows, trees], "tree row/tree pairs")?;
	for (name, elements) in [
		("flattened feature image", feature_elements),
		("flattened split image", split_elements),
		("flattened leaf-value image", leaf_elements),
		("row/tree pair image", pair_elements),
	] {
		if elements > i32::MAX as u64 {
			return Err(tree_error(
				OperationErrorKind::UnsupportedConcreteShape,
				format!("{name} exceeds the checked int32 traversal domain"),
			));
		}
	}
	let expanded = checked_product(&[pair_elements, outputs], "tree expanded contributions")?;
	let reduced = checked_product(&[rows, outputs], "tree reduced predictions")?;
	let depth = u64::from(depth);
	let pair_tensors = checked_sum(
		&[
			3,
			checked_product(
				&[INTERMEDIATES_PER_DEPTH, depth],
				"tree depth intermediates",
			)?,
		],
		"tree pair tensor count",
	)?;
	let workspace_elements = checked_sum(
		&[
			checked_product(&[pair_tensors, pair_elements], "tree pair workspace")?,
			outputs,
			checked_product(&[2, expanded], "tree expanded workspace")?,
			reduced,
		],
		"tree traversal workspace elements",
	)?;
	Ok(TreeEnsembleInferenceRequirements {
		internal_nodes_per_tree: internal_nodes,
		leaves_per_tree: leaves,
		intermediate_values: FIXED_INTERMEDIATES + INTERMEDIATES_PER_DEPTH * depth,
		kernels: FIXED_KERNELS + KERNELS_PER_DEPTH * depth,
		workspace_bytes: ByteCount::new(checked_product(
			&[workspace_elements, 4],
			"tree traversal workspace bytes",
		)?),
	})
}

pub fn materialize_tree_ensemble_inference(
	request: &TreeEnsembleInferenceRequest,
) -> OperationResult<TreeEnsembleInferenceMaterialization> {
	let requirements = validate_request(request)?;
	validate_resources(request, requirements)?;
	let pair_shape = shape(&[request.rows, request.trees, 1])?;
	let mut emitter = TreeEmitter::new(request, requirements)?;
	let pair_positions = emitter.intermediate(DType::I32, pair_shape.clone())?;
	emitter.index_map(pair_positions, 0, 1)?;
	let mut nodes = emitter.intermediate(DType::I32, pair_shape.clone())?;
	emitter.index_map(nodes, 0, 0)?;

	for _ in 0..request.depth {
		let split_indices = emitter.intermediate(DType::I32, pair_shape.clone())?;
		emitter.elementwise(
			vec![pair_positions, nodes],
			vec![split_indices],
			split_index_program(request.trees, requirements.internal_nodes_per_tree)?,
		)?;
		let selected_features = emitter.intermediate(DType::I32, pair_shape.clone())?;
		emitter.gather(request.split_features.id, split_indices, selected_features)?;
		let selected_thresholds = emitter.intermediate(DType::F32, pair_shape.clone())?;
		emitter.gather(
			request.split_thresholds.id,
			split_indices,
			selected_thresholds,
		)?;
		let feature_indices = emitter.intermediate(DType::I32, pair_shape.clone())?;
		emitter.elementwise(
			vec![pair_positions, selected_features],
			vec![feature_indices],
			feature_index_program(request.trees, request.feature_width)?,
		)?;
		let selected_values = emitter.intermediate(DType::F32, pair_shape.clone())?;
		emitter.gather(request.features.id, feature_indices, selected_values)?;
		let next_nodes = emitter.intermediate(DType::I32, pair_shape.clone())?;
		emitter.elementwise(
			vec![selected_values, selected_thresholds, nodes],
			vec![next_nodes],
			next_node_program(requirements.internal_nodes_per_tree)?,
		)?;
		nodes = next_nodes;
	}

	let leaf_bases = emitter.intermediate(DType::I32, pair_shape)?;
	emitter.elementwise(
		vec![pair_positions, nodes],
		vec![leaf_bases],
		leaf_base_program(
			request.trees,
			requirements.internal_nodes_per_tree,
			requirements.leaves_per_tree,
			request.outputs,
		)?,
	)?;
	let output_offsets = emitter.intermediate(DType::I32, shape(&[1, 1, request.outputs])?)?;
	emitter.index_map(output_offsets, 0, 1)?;
	let contribution_shape = shape(&[request.rows, request.trees, request.outputs])?;
	let leaf_indices = emitter.intermediate(DType::I32, contribution_shape.clone())?;
	emitter.elementwise(
		vec![leaf_bases, output_offsets],
		vec![leaf_indices],
		leaf_index_program(request.outputs)?,
	)?;
	let contributions = emitter.intermediate(DType::F32, contribution_shape)?;
	emitter.gather(request.leaf_values.id, leaf_indices, contributions)?;
	let sums = emitter.intermediate(DType::F32, shape(&[request.rows, request.outputs])?)?;
	emitter.emit(
		vec![contributions],
		vec![sums],
		PrimitiveKind::Reduce(Reduce {
			operator: ReduceOperator::Sum,
			axes: AxisSet::new(vec![1]).map_err(graph_error)?,
			keep_dimensions: false,
			result: ReduceResult::Value,
			tree_lanes: request.tree_lanes,
		}),
	)?;
	emitter.elementwise(
		vec![sums],
		vec![request.predictions.id],
		scale_program(request.scale)?,
	)?;
	emitter.finish()
}

/// Append a validated saved-tree traversal to a caller-owned graph.
///
/// Every boundary tensor must already exist with the same storage contract.
/// Only traversal intermediates and kernels are appended.
pub fn append_tree_ensemble_inference(
	graph: &mut CalculationGraph,
	request: &TreeEnsembleInferenceRequest,
) -> OperationResult<TreeEnsembleInferenceMaterialization> {
	validate_boundary_storage(graph, request)?;
	let materialized = materialize_tree_ensemble_inference(request)?;
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
		return Err(tree_error(
			OperationErrorKind::IdentityNamespaceOverlap,
			format!("tree intermediate {value} already exists in caller graph"),
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
		return Err(tree_error(
			OperationErrorKind::IdentityNamespaceOverlap,
			format!("tree kernel {kernel} already exists in caller graph"),
		));
	}
	if let Some(node) = graph
		.nodes
		.iter()
		.find(|node| node.kernel.outputs.contains(&request.predictions.id))
	{
		return Err(tree_error(
			OperationErrorKind::GraphMaterializationFailed,
			format!(
				"caller graph kernel {} already produces tree predictions",
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

fn validate_request(request: &TreeEnsembleInferenceRequest) -> OperationResult<TreeEnsembleInferenceRequirements> {
	for tensor in [
		&request.features,
		&request.split_features,
		&request.split_thresholds,
		&request.leaf_values,
		&request.predictions,
	] {
		tensor.validate().map_err(graph_error)?;
	}
	let requirements = tree_ensemble_inference_requirements(
		request.rows,
		request.feature_width,
		request.trees,
		request.depth,
		request.outputs,
	)?;
	let feature_elements = checked_product(
		&[request.rows, request.feature_width],
		"tree feature elements",
	)?;
	let split_elements = checked_product(
		&[request.trees, requirements.internal_nodes_per_tree],
		"tree split elements",
	)?;
	let leaf_elements = checked_product(
		&[request.trees, requirements.leaves_per_tree, request.outputs],
		"tree leaf-value elements",
	)?;
	for (name, tensor, dtype, extents) in [
		(
			"features",
			&request.features,
			DType::F32,
			vec![feature_elements],
		),
		(
			"split features",
			&request.split_features,
			DType::I32,
			vec![split_elements],
		),
		(
			"split thresholds",
			&request.split_thresholds,
			DType::F32,
			vec![split_elements],
		),
		(
			"leaf values",
			&request.leaf_values,
			DType::F32,
			vec![leaf_elements],
		),
		(
			"predictions",
			&request.predictions,
			DType::F32,
			vec![request.rows, request.outputs],
		),
	] {
		if tensor.dtype != dtype || tensor.shape.extents() != extents {
			return Err(tree_error(
				OperationErrorKind::UnsupportedConcreteShape,
				format!("{name} must be {dtype:?} {extents:?}"),
			));
		}
	}
	if !request.scale.is_finite() {
		return Err(tree_error(
			OperationErrorKind::InvalidMaterializationRequest,
			"tree ensemble scale must be finite",
		));
	}
	if request.tree_lanes == 0 || request.tree_lanes > MAXIMUM_TREE_LANES || !request.tree_lanes.is_power_of_two() {
		return Err(tree_error(
			OperationErrorKind::InvalidMaterializationRequest,
			format!("tree reduction lanes must be a power of two in 1..={MAXIMUM_TREE_LANES}"),
		));
	}
	Ok(requirements)
}

fn validate_boundary_storage(graph: &CalculationGraph, request: &TreeEnsembleInferenceRequest) -> OperationResult<()> {
	let mut ids = BTreeSet::new();
	for tensor in &graph.tensors {
		if !ids.insert(tensor.id) {
			return Err(tree_error(
				OperationErrorKind::GraphMaterializationFailed,
				format!("caller graph repeats tensor {}", tensor.id),
			));
		}
	}
	for declared in [
		&request.features,
		&request.split_features,
		&request.split_thresholds,
		&request.leaf_values,
		&request.predictions,
	] {
		let existing = graph
			.tensors
			.iter()
			.find(|tensor| tensor.id == declared.id)
			.ok_or_else(|| {
				tree_error(
					OperationErrorKind::InvalidMaterializationRequest,
					format!(
						"caller graph does not declare tree boundary tensor {}",
						declared.id
					),
				)
			})?;
		if !same_storage_contract(existing, declared) {
			return Err(tree_error(
				OperationErrorKind::InvalidMaterializationRequest,
				format!(
					"caller graph tensor {} differs from the tree storage contract",
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

fn validate_resources(
	request: &TreeEnsembleInferenceRequest,
	requirements: TreeEnsembleInferenceRequirements,
) -> OperationResult<()> {
	if requirements.intermediate_values > request.identity_namespace.value_capacity() {
		return Err(tree_error(
			OperationErrorKind::IdentityNamespaceExhausted,
			format!(
				"tree traversal needs {} intermediate identities, namespace reserves {}",
				requirements.intermediate_values,
				request.identity_namespace.value_capacity()
			),
		));
	}
	if requirements.kernels > request.identity_namespace.kernel_capacity() {
		return Err(tree_error(
			OperationErrorKind::IdentityNamespaceExhausted,
			format!(
				"tree traversal needs {} kernel identities, namespace reserves {}",
				requirements.kernels,
				request.identity_namespace.kernel_capacity()
			),
		));
	}
	if requirements.workspace_bytes.get() > request.workspace_limit.get() {
		return Err(tree_error(
			OperationErrorKind::WorkspaceLimitExceeded,
			format!(
				"tree traversal needs {} workspace bytes, limit is {}",
				requirements.workspace_bytes.get(),
				request.workspace_limit.get()
			),
		));
	}
	Ok(())
}

struct TreeEmitter {
	tensors: Vec<Tensor>,
	nodes: Vec<CalculationNode>,
	intermediate_values: Vec<ValueId>,
	kernels: Vec<KernelTemplateId>,
	next_value: u64,
	value_end: u64,
	next_kernel: u64,
	kernel_end: u64,
	workspace_bytes: u64,
	requirements: TreeEnsembleInferenceRequirements,
	identity_namespace: IdentityNamespace,
}

impl TreeEmitter {
	fn new(
		request: &TreeEnsembleInferenceRequest,
		requirements: TreeEnsembleInferenceRequirements,
	) -> OperationResult<Self> {
		let first_value = request.identity_namespace.first_value().get();
		let value_end = first_value
			.checked_add(request.identity_namespace.value_capacity())
			.ok_or_else(|| {
				tree_error(
					OperationErrorKind::IdentityNamespaceExhausted,
					"tree value namespace overflows u64",
				)
			})?;
		let first_kernel = request.identity_namespace.first_kernel().get();
		let kernel_end = first_kernel
			.checked_add(request.identity_namespace.kernel_capacity())
			.ok_or_else(|| {
				tree_error(
					OperationErrorKind::IdentityNamespaceExhausted,
					"tree kernel namespace overflows u64",
				)
			})?;
		let mut boundaries = BTreeMap::<ValueId, Tensor>::new();
		for source in [
			&request.features,
			&request.split_features,
			&request.split_thresholds,
			&request.leaf_values,
		] {
			let mut tensor = source.clone();
			tensor.external_input = true;
			tensor.external_output = false;
			insert_boundary(&mut boundaries, tensor)?;
		}
		let mut predictions = request.predictions.clone();
		predictions.external_input = false;
		predictions.external_output = true;
		if boundaries.contains_key(&predictions.id) {
			return Err(tree_error(
				OperationErrorKind::InvalidMaterializationRequest,
				"tree predictions must not alias an input tensor identity",
			));
		}
		insert_boundary(&mut boundaries, predictions)?;
		if let Some(tensor) = boundaries
			.values()
			.find(|tensor| tensor.id.get() >= first_value && tensor.id.get() < value_end)
		{
			return Err(tree_error(
				OperationErrorKind::IdentityNamespaceOverlap,
				format!(
					"boundary tensor {} overlaps tree intermediate range [{first_value}, {value_end})",
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
			return Err(tree_error(
				OperationErrorKind::IdentityNamespaceExhausted,
				"tree intermediate identity range is exhausted",
			));
		}
		let value = ValueId::new(self.next_value);
		self.next_value += 1;
		let tensor = Tensor::contiguous(value, dtype, shape, false, false).map_err(graph_error)?;
		self.workspace_bytes = self
			.workspace_bytes
			.checked_add(tensor.storage_bytes.get())
			.ok_or_else(|| {
				tree_error(
					OperationErrorKind::WorkspaceArithmeticOverflow,
					"tree workspace total overflows u64",
				)
			})?;
		self.intermediate_values.push(value);
		self.tensors.push(tensor);
		Ok(value)
	}

	fn emit(&mut self, inputs: Vec<ValueId>, outputs: Vec<ValueId>, kind: PrimitiveKind) -> OperationResult<()> {
		if self.next_kernel == self.kernel_end {
			return Err(tree_error(
				OperationErrorKind::IdentityNamespaceExhausted,
				"tree kernel identity range is exhausted",
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

	fn finish(self) -> OperationResult<TreeEnsembleInferenceMaterialization> {
		if self.intermediate_values.len() as u64 != self.requirements.intermediate_values
			|| self.kernels.len() as u64 != self.requirements.kernels
			|| self.workspace_bytes != self.requirements.workspace_bytes.get()
		{
			return Err(tree_error(
				OperationErrorKind::WorkspaceFormulaMismatch,
				format!(
					"tree traversal emitted {}/{}/{} values/kernels/bytes, expected {}/{}/{}",
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
		Ok(TreeEnsembleInferenceMaterialization {
			graph,
			intermediate_values: self.intermediate_values,
			kernels: self.kernels,
			internal_nodes_per_tree: self.requirements.internal_nodes_per_tree,
			leaves_per_tree: self.requirements.leaves_per_tree,
			workspace_bytes: ByteCount::new(self.workspace_bytes),
			identity_namespace: self.identity_namespace,
		})
	}
}

fn split_index_program(trees: u64, internal_nodes: u64) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder()?;
	let pair = scalar_input(&mut builder, DType::I32)?;
	let node = scalar_input(&mut builder, DType::I32)?;
	let trees = scalar_i32(&mut builder, trees, "tree count")?;
	let internal_nodes = scalar_i32(&mut builder, internal_nodes, "internal node count")?;
	require_i32_range(&mut builder, node, internal_nodes)?;
	let tree = scalar_binary(&mut builder, ScalarOpcode::Remainder, pair, trees)?;
	let base = scalar_binary(&mut builder, ScalarOpcode::Multiply, tree, internal_nodes)?;
	let index = scalar_binary(&mut builder, ScalarOpcode::Add, base, node)?;
	scalar_finish(builder, &[index])
}

fn feature_index_program(trees: u64, feature_width: u64) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder()?;
	let pair = scalar_input(&mut builder, DType::I32)?;
	let feature = scalar_input(&mut builder, DType::I32)?;
	let trees = scalar_i32(&mut builder, trees, "tree count")?;
	let feature_width = scalar_i32(&mut builder, feature_width, "feature width")?;
	require_i32_range(&mut builder, feature, feature_width)?;
	let row = scalar_binary(&mut builder, ScalarOpcode::Divide, pair, trees)?;
	let base = scalar_binary(&mut builder, ScalarOpcode::Multiply, row, feature_width)?;
	let index = scalar_binary(&mut builder, ScalarOpcode::Add, base, feature)?;
	scalar_finish(builder, &[index])
}

fn next_node_program(internal_nodes: u64) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder()?;
	let value = scalar_input(&mut builder, DType::F32)?;
	let threshold = scalar_input(&mut builder, DType::F32)?;
	let node = scalar_input(&mut builder, DType::I32)?;
	let internal_nodes = scalar_i32(&mut builder, internal_nodes, "internal node count")?;
	require_i32_range(&mut builder, node, internal_nodes)?;
	require_finite(&mut builder, value)?;
	require_finite(&mut builder, threshold)?;
	let right = scalar_binary(&mut builder, ScalarOpcode::GreaterThan, value, threshold)?;
	let two = builder.i32(2).map_err(graph_error)?;
	let one = builder.i32(1).map_err(graph_error)?;
	let doubled = scalar_binary(&mut builder, ScalarOpcode::Multiply, node, two)?;
	let left = scalar_binary(&mut builder, ScalarOpcode::Add, doubled, one)?;
	let next = scalar_binary(&mut builder, ScalarOpcode::Add, left, right)?;
	scalar_finish(builder, &[next])
}

fn leaf_base_program(
	trees: u64,
	internal_nodes: u64,
	leaves: u64,
	outputs: u64,
) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder()?;
	let pair = scalar_input(&mut builder, DType::I32)?;
	let node = scalar_input(&mut builder, DType::I32)?;
	let trees = scalar_i32(&mut builder, trees, "tree count")?;
	let internal_nodes = scalar_i32(&mut builder, internal_nodes, "internal node count")?;
	let leaves = scalar_i32(&mut builder, leaves, "leaf count")?;
	let outputs = scalar_i32(&mut builder, outputs, "output width")?;
	let leaf_limit = scalar_binary(&mut builder, ScalarOpcode::Add, internal_nodes, leaves)?;
	require_i32_interval(&mut builder, node, internal_nodes, leaf_limit)?;
	let tree = scalar_binary(&mut builder, ScalarOpcode::Remainder, pair, trees)?;
	let leaf = scalar_binary(&mut builder, ScalarOpcode::Subtract, node, internal_nodes)?;
	let tree_base = scalar_binary(&mut builder, ScalarOpcode::Multiply, tree, leaves)?;
	let tree_leaf = scalar_binary(&mut builder, ScalarOpcode::Add, tree_base, leaf)?;
	let base = scalar_binary(&mut builder, ScalarOpcode::Multiply, tree_leaf, outputs)?;
	scalar_finish(builder, &[base])
}

fn leaf_index_program(outputs: u64) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder()?;
	let base = scalar_input(&mut builder, DType::I32)?;
	let output = scalar_input(&mut builder, DType::I32)?;
	let outputs = scalar_i32(&mut builder, outputs, "output width")?;
	require_i32_range(&mut builder, output, outputs)?;
	let index = scalar_binary(&mut builder, ScalarOpcode::Add, base, output)?;
	scalar_finish(builder, &[index])
}

fn scale_program(scale: f32) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder()?;
	let value = scalar_input(&mut builder, DType::F32)?;
	let scale = builder.f32(scale).map_err(graph_error)?;
	let scaled = scalar_binary(&mut builder, ScalarOpcode::Multiply, value, scale)?;
	scalar_finish(builder, &[scaled])
}

fn require_finite(builder: &mut ScalarProgramBuilder, value: ScalarExpression) -> OperationResult<()> {
	let finite = builder
		.unary(ScalarOpcode::IsFinite, value)
		.map_err(graph_error)?;
	let _ = builder
		.unary(ScalarOpcode::Require, finite)
		.map_err(graph_error)?;
	Ok(())
}

fn require_i32_range(
	builder: &mut ScalarProgramBuilder,
	value: ScalarExpression,
	upper_exclusive: ScalarExpression,
) -> OperationResult<()> {
	let zero = builder.i32(0).map_err(graph_error)?;
	require_i32_interval(builder, value, zero, upper_exclusive)
}

fn require_i32_interval(
	builder: &mut ScalarProgramBuilder,
	value: ScalarExpression,
	lower_inclusive: ScalarExpression,
	upper_exclusive: ScalarExpression,
) -> OperationResult<()> {
	let lower = scalar_binary(
		builder,
		ScalarOpcode::GreaterThanOrEqual,
		value,
		lower_inclusive,
	)?;
	let upper = scalar_binary(builder, ScalarOpcode::LessThan, value, upper_exclusive)?;
	let valid = scalar_binary(builder, ScalarOpcode::BitAnd, lower, upper)?;
	let _ = builder
		.unary(ScalarOpcode::Require, valid)
		.map_err(graph_error)?;
	Ok(())
}

fn scalar_builder() -> OperationResult<ScalarProgramBuilder> {
	ScalarProgramBuilder::new().map_err(graph_error)
}

fn scalar_input(builder: &mut ScalarProgramBuilder, dtype: DType) -> OperationResult<ScalarExpression> {
	builder.input(dtype).map_err(graph_error)
}

fn scalar_i32(builder: &mut ScalarProgramBuilder, value: u64, role: &str) -> OperationResult<ScalarExpression> {
	let value = i32::try_from(value).map_err(|error| {
		tree_error(
			OperationErrorKind::UnsupportedConcreteShape,
			format!("{role} cannot be represented by i32: {error}"),
		)
	})?;
	builder.i32(value).map_err(graph_error)
}

fn scalar_binary(
	builder: &mut ScalarProgramBuilder,
	opcode: ScalarOpcode,
	left: ScalarExpression,
	right: ScalarExpression,
) -> OperationResult<ScalarExpression> {
	builder.binary(opcode, left, right).map_err(graph_error)
}

fn scalar_finish(
	builder: ScalarProgramBuilder,
	outputs: &[ScalarExpression],
) -> OperationResult<recipe_core::ScalarProgram> {
	builder.finish(outputs).map_err(graph_error)
}

fn shape(extents: &[u64]) -> OperationResult<Shape> {
	Shape::new(extents.to_vec()).map_err(graph_error)
}

fn insert_boundary(boundaries: &mut BTreeMap<ValueId, Tensor>, tensor: Tensor) -> OperationResult<()> {
	match boundaries.get(&tensor.id) {
		Some(existing)
			if existing.dtype == tensor.dtype
				&& existing.shape == tensor.shape
				&& existing.layout == tensor.layout
				&& existing.storage_bytes == tensor.storage_bytes =>
		{
			Ok(())
		}
		Some(_) => Err(tree_error(
			OperationErrorKind::InvalidMaterializationRequest,
			format!(
				"tree boundary tensor {} has conflicting contracts",
				tensor.id
			),
		)),
		None => {
			boundaries.insert(tensor.id, tensor);
			Ok(())
		}
	}
}

fn forbidden_aliases(input_count: usize, output_count: usize) -> Vec<PrimitiveAliasRule> {
	(0..input_count)
		.flat_map(|input| {
			(0..output_count).map(move |output| PrimitiveAliasRule {
				input,
				output,
				permission: AliasPermission::Forbidden,
			})
		})
		.collect()
}

fn checked_product(values: &[u64], role: &str) -> OperationResult<u64> {
	values.iter().copied().try_fold(1_u64, |product, value| {
		product.checked_mul(value).ok_or_else(|| {
			tree_error(
				OperationErrorKind::WorkspaceArithmeticOverflow,
				format!("{role} overflowed u64"),
			)
		})
	})
}

fn checked_sum(values: &[u64], role: &str) -> OperationResult<u64> {
	values.iter().copied().try_fold(0_u64, |sum, value| {
		sum.checked_add(value).ok_or_else(|| {
			tree_error(
				OperationErrorKind::WorkspaceArithmeticOverflow,
				format!("{role} overflowed u64"),
			)
		})
	})
}

fn graph_error(error: impl ToString) -> OperationError {
	tree_error(
		OperationErrorKind::GraphMaterializationFailed,
		error.to_string(),
	)
}

fn tree_error(kind: OperationErrorKind, detail: impl Into<String>) -> OperationError {
	OperationError {
		operation: None,
		kind,
		detail: detail.into(),
	}
}
