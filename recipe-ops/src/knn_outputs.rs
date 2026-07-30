use std::collections::{BTreeMap, BTreeSet};

use recipe_core::{AliasPermission, ByteCount, DType, KernelTemplateId, ScalarOpcode, ValueId};
use recipe_language::{
	AtomicOrdering, AxisSet, CalculationGraph, CalculationNode, Contraction, Elementwise, Gather, Histogram,
	IndexBounds, IndexMap, PrimitiveAliasRule, PrimitiveKernel, PrimitiveKind, Reduce, ReduceOperator, ReduceResult,
	ScalarExpression, ScalarProgramBuilder, Shape, Sort, SortDirection, Tensor,
};

use crate::{IdentityNamespace, OperationError, OperationErrorKind, OperationResult};

const BASE_INTERMEDIATES: u64 = 6;
const BASE_KERNELS: u64 = 6;
const NUMERIC_INTERMEDIATES: u64 = 10;
const NUMERIC_KERNELS: u64 = 10;
const CATEGORICAL_INTERMEDIATES: u64 = 17;
const CATEGORICAL_KERNELS: u64 = 16;
const MAXIMUM_TREE_LANES: u32 = 1_024;
const MAXIMUM_EXACT_F32_INTEGER: u64 = 16_777_216;

/// Static semantic contract for one declared KNN output.
///
/// `known_references` is the number of true entries in the corresponding
/// runtime mask. The emitted graph verifies that count before selecting
/// neighbors. Categorical ties choose the lowest canonical category code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KnnOutputSpec {
	Numeric { known_references: u64 },
	Categorical { known_references: u64, classes: u64 },
}

impl KnnOutputSpec {
	#[must_use]
	pub const fn known_references(self) -> u64 {
		match self {
			Self::Numeric { known_references }
			| Self::Categorical {
				known_references, ..
			} => known_references,
		}
	}
}

/// One typed target vector and its prediction boundary.
///
/// Numeric targets retain f32 values and use an unweighted mean. Categorical
/// targets retain canonical int32 dictionary codes and use an unweighted mode.
/// Every target owns a separate known-value mask, so unrelated semantic spaces
/// are never concatenated or averaged together.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KnnOutputRequest {
	Numeric {
		reference_values: Tensor,
		known: Tensor,
		predictions: Tensor,
		known_references: u64,
	},
	Categorical {
		reference_codes: Tensor,
		known: Tensor,
		predictions: Tensor,
		known_references: u64,
		classes: u64,
	},
}

impl KnnOutputRequest {
	#[must_use]
	pub const fn spec(&self) -> KnnOutputSpec {
		match self {
			Self::Numeric {
				known_references, ..
			} => KnnOutputSpec::Numeric {
				known_references: *known_references,
			},
			Self::Categorical {
				known_references,
				classes,
				..
			} => KnnOutputSpec::Categorical {
				known_references: *known_references,
				classes: *classes,
			},
		}
	}

	fn reference_values(&self) -> &Tensor {
		match self {
			Self::Numeric {
				reference_values, ..
			} => reference_values,
			Self::Categorical {
				reference_codes, ..
			} => reference_codes,
		}
	}

	fn known(&self) -> &Tensor {
		match self {
			Self::Numeric { known, .. } | Self::Categorical { known, .. } => known,
		}
	}

	fn predictions(&self) -> &Tensor {
		match self {
			Self::Numeric { predictions, .. } | Self::Categorical { predictions, .. } => predictions,
		}
	}
}

/// One all-output KNN graph request.
///
/// Query and reference features are normalized f32 matrices. Distances are
/// rooted L2. Each declared output independently masks unknown references,
/// selects the nearest `min(neighbors, known_references)` rows, and aggregates
/// according to its semantic dtype. Stable distance ties retain prepared
/// reference-row order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnnAllOutputRequest {
	pub query_features: Tensor,
	pub reference_features: Tensor,
	pub outputs: Vec<KnnOutputRequest>,
	pub neighbors: u64,
	pub tree_lanes: u32,
	pub identity_namespace: IdentityNamespace,
	pub workspace_limit: ByteCount,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KnnOutputRequirement {
	pub effective_neighbors: u64,
	pub classes: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnnAllOutputRequirements {
	pub outputs: Vec<KnnOutputRequirement>,
	pub intermediate_values: u64,
	pub kernels: u64,
	pub workspace_bytes: ByteCount,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnnAllOutputMaterialization {
	pub graph: CalculationGraph,
	pub intermediate_values: Vec<ValueId>,
	pub kernels: Vec<KernelTemplateId>,
	pub output_requirements: Vec<KnnOutputRequirement>,
	pub workspace_bytes: ByteCount,
	pub identity_namespace: IdentityNamespace,
}

pub fn knn_all_output_requirements(
	queries: u64,
	reference_rows: u64,
	dimensions: u64,
	neighbors: u64,
	outputs: &[KnnOutputSpec],
) -> OperationResult<KnnAllOutputRequirements> {
	for (name, value) in [
		("query row count", queries),
		("reference row count", reference_rows),
		("feature dimension", dimensions),
		("neighbor count", neighbors),
	] {
		if value == 0 {
			return Err(knn_error(
				OperationErrorKind::UnsupportedConcreteShape,
				format!("KNN {name} must be nonzero"),
			));
		}
	}
	if outputs.is_empty() {
		return Err(knn_error(
			OperationErrorKind::UnsupportedConcreteShape,
			"KNN requires at least one declared output",
		));
	}
	if reference_rows > i32::MAX as u64 {
		return Err(knn_error(
			OperationErrorKind::UnsupportedConcreteShape,
			"KNN reference rows exceed the checked int32 sort-index domain",
		));
	}
	let query_features = checked_product(&[queries, dimensions], "KNN query feature elements")?;
	let reference_features = checked_product(
		&[reference_rows, dimensions],
		"KNN reference feature elements",
	)?;
	let pairwise = checked_product(&[queries, reference_rows], "KNN pairwise elements")?;
	let base_workspace = checked_sum(
		&[
			query_features,
			reference_features,
			queries,
			reference_rows,
			checked_product(&[2, pairwise], "KNN base pairwise workspace")?,
		],
		"KNN base workspace",
	)?;
	let mut workspace_elements = base_workspace;
	let mut intermediate_values = BASE_INTERMEDIATES;
	let mut kernels = BASE_KERNELS;
	let mut requirements = Vec::with_capacity(outputs.len());
	for (output_index, output) in outputs.iter().copied().enumerate() {
		let known = output.known_references();
		if known == 0 || known > reference_rows {
			return Err(knn_error(
				OperationErrorKind::UnsupportedConcreteShape,
				format!(
					"KNN output {output_index} known-reference count must be in 1..={reference_rows}, got {known}"
				),
			));
		}
		let effective_neighbors = neighbors.min(known);
		let selected = checked_product(
			&[queries, effective_neighbors],
			"KNN selected neighbor elements",
		)?;
		let common = checked_sum(
			&[
				2,
				checked_product(&[3, pairwise], "KNN per-output pairwise workspace")?,
				effective_neighbors,
				selected,
			],
			"KNN per-output common workspace",
		)?;
		let (classes, extra, values, output_kernels) = match output {
			KnnOutputSpec::Numeric { .. } => {
				if effective_neighbors > MAXIMUM_EXACT_F32_INTEGER {
					return Err(knn_error(
						OperationErrorKind::UnsupportedConcreteShape,
						format!(
							"KNN output {output_index} numeric neighbor count exceeds the exact f32 integer domain"
						),
					));
				}
				(
					None,
					checked_sum(
						&[reference_rows, selected, queries],
						"KNN numeric aggregation workspace",
					)?,
					NUMERIC_INTERMEDIATES,
					NUMERIC_KERNELS,
				)
			}
			KnnOutputSpec::Categorical { classes, .. } => {
				if classes == 0 || classes > i32::MAX as u64 {
					return Err(knn_error(
						OperationErrorKind::UnsupportedConcreteShape,
						format!("KNN output {output_index} class count must be in 1..=i32::MAX"),
					));
				}
				let histogram_bins = checked_product(&[queries, classes], "KNN categorical histogram bins")?;
				if histogram_bins > i32::MAX as u64 {
					return Err(knn_error(
						OperationErrorKind::UnsupportedConcreteShape,
						format!("KNN output {output_index} row/class histogram exceeds i32::MAX bins"),
					));
				}
				(
					Some(classes),
					checked_sum(
						&[
							reference_rows,
							checked_product(&[2, selected], "KNN categorical selected workspace")?,
							queries,
							checked_product(&[5, histogram_bins], "KNN categorical histogram workspace")?,
							1,
						],
						"KNN categorical aggregation workspace",
					)?,
					CATEGORICAL_INTERMEDIATES,
					CATEGORICAL_KERNELS,
				)
			}
		};
		workspace_elements = checked_sum(&[workspace_elements, common, extra], "KNN total workspace")?;
		intermediate_values = checked_sum(&[intermediate_values, values], "KNN intermediate count")?;
		kernels = checked_sum(&[kernels, output_kernels], "KNN kernel count")?;
		requirements.push(KnnOutputRequirement {
			effective_neighbors,
			classes,
		});
	}
	Ok(KnnAllOutputRequirements {
		outputs: requirements,
		intermediate_values,
		kernels,
		workspace_bytes: ByteCount::new(checked_product(
			&[workspace_elements, 4],
			"KNN workspace bytes",
		)?),
	})
}

pub fn materialize_knn_all_outputs(request: &KnnAllOutputRequest) -> OperationResult<KnnAllOutputMaterialization> {
	let requirements = validate_request(request)?;
	validate_resources(request, &requirements)?;
	let queries = request.query_features.shape.extents()[0];
	let dimensions = request.query_features.shape.extents()[1];
	let reference_rows = request.reference_features.shape.extents()[0];
	let mut emitter = KnnAllOutputEmitter::new(request, requirements.clone())?;

	let query_squared = emitter.intermediate(DType::F32, shape(&[queries, dimensions])?)?;
	emitter.elementwise(
		vec![request.query_features.id],
		vec![query_squared],
		square_program()?,
	)?;
	let reference_squared = emitter.intermediate(DType::F32, shape(&[reference_rows, dimensions])?)?;
	emitter.elementwise(
		vec![request.reference_features.id],
		vec![reference_squared],
		square_program()?,
	)?;
	let query_norms = emitter.intermediate(DType::F32, shape(&[queries, 1])?)?;
	emitter.reduce(query_squared, query_norms, &[1], true, request.tree_lanes)?;
	let reference_norms = emitter.intermediate(DType::F32, shape(&[reference_rows])?)?;
	emitter.reduce(
		reference_squared,
		reference_norms,
		&[1],
		false,
		request.tree_lanes,
	)?;
	let products = emitter.intermediate(DType::F32, shape(&[queries, reference_rows])?)?;
	emitter.emit(
		vec![request.query_features.id, request.reference_features.id],
		vec![products],
		PrimitiveKind::Contraction(Contraction {
			batch_axes: Vec::new(),
			contract_axes: vec![(1, 1)],
		}),
	)?;
	let distances = emitter.intermediate(DType::F32, shape(&[queries, reference_rows])?)?;
	emitter.elementwise(
		vec![query_norms, reference_norms, products],
		vec![distances],
		pairwise_l2_program()?,
	)?;

	for (output, output_requirements) in request.outputs.iter().zip(&requirements.outputs) {
		let validated_reference = match output {
			KnnOutputRequest::Numeric {
				reference_values, ..
			} => {
				let validated = emitter.intermediate(DType::F32, shape(&[reference_rows])?)?;
				emitter.elementwise(
					vec![reference_values.id],
					vec![validated],
					finite_identity_program()?,
				)?;
				validated
			}
			KnnOutputRequest::Categorical {
				reference_codes,
				classes,
				..
			} => {
				let validated = emitter.intermediate(DType::I32, shape(&[reference_rows])?)?;
				emitter.elementwise(
					vec![reference_codes.id],
					vec![validated],
					categorical_code_program(*classes)?,
				)?;
				validated
			}
		};
		let known_count = emitter.intermediate(DType::I32, shape(&[1])?)?;
		emitter.reduce(
			output.known().id,
			known_count,
			&[0],
			false,
			request.tree_lanes,
		)?;
		let validated_count = emitter.intermediate(DType::I32, shape(&[1])?)?;
		emitter.elementwise(
			vec![known_count],
			vec![validated_count],
			known_count_program(output.spec().known_references())?,
		)?;
		let masked_distances = emitter.intermediate(DType::F32, shape(&[queries, reference_rows])?)?;
		emitter.elementwise(
			vec![distances, output.known().id],
			vec![masked_distances],
			masked_distance_program()?,
		)?;
		let sorted_distances = emitter.intermediate(DType::F32, shape(&[queries, reference_rows])?)?;
		let sorted_indices = emitter.intermediate(DType::I32, shape(&[queries, reference_rows])?)?;
		emitter.emit(
			vec![masked_distances],
			vec![sorted_distances, sorted_indices],
			PrimitiveKind::Sort(Sort {
				axis: 1,
				direction: SortDirection::Ascending,
				stable: true,
				emit_indices: true,
			}),
		)?;
		let neighbors = output_requirements.effective_neighbors;
		let prefix = emitter.intermediate(DType::I32, shape(&[neighbors])?)?;
		emitter.index_map(prefix, 0, 1)?;
		let neighbor_indices = emitter.intermediate(DType::I32, shape(&[queries, neighbors])?)?;
		emitter.gather(sorted_indices, prefix, neighbor_indices, 1)?;

		match output {
			KnnOutputRequest::Numeric { predictions, .. } => {
				let values = emitter.intermediate(DType::F32, shape(&[queries, neighbors])?)?;
				emitter.gather(validated_reference, neighbor_indices, values, 0)?;
				let sums = emitter.intermediate(DType::F32, shape(&[queries, 1])?)?;
				emitter.reduce(values, sums, &[1], true, request.tree_lanes)?;
				emitter.elementwise(
					vec![sums],
					vec![predictions.id],
					finite_divide_program(neighbors)?,
				)?;
			}
			KnnOutputRequest::Categorical {
				predictions,
				classes,
				..
			} => {
				let codes = emitter.intermediate(DType::I32, shape(&[queries, neighbors])?)?;
				emitter.gather(validated_reference, neighbor_indices, codes, 0)?;
				let row_bases = emitter.intermediate(DType::I32, shape(&[queries, 1])?)?;
				emitter.index_map(
					row_bases,
					0,
					i32::try_from(*classes).map_err(|error| {
						knn_error(
							OperationErrorKind::UnsupportedConcreteShape,
							format!("KNN class count cannot be represented by i32: {error}"),
						)
					})?,
				)?;
				let bin_indices = emitter.intermediate(DType::I32, shape(&[queries, neighbors])?)?;
				emitter.elementwise(
					vec![row_bases, codes],
					vec![bin_indices],
					categorical_bin_program(*classes)?,
				)?;
				let bins = checked_product(&[queries, *classes], "KNN categorical bins")?;
				let flat_counts = emitter.intermediate(DType::I32, shape(&[bins])?)?;
				emitter.emit(
					vec![bin_indices],
					vec![flat_counts],
					PrimitiveKind::Histogram(Histogram {
						bins: u32::try_from(bins).map_err(|error| {
							knn_error(
								OperationErrorKind::UnsupportedConcreteShape,
								format!(
									"KNN histogram bin count cannot be represented by u32: {error}"
								),
							)
						})?,
						weighted: false,
						ordering: AtomicOrdering::Relaxed,
					}),
				)?;
				let lookups = emitter.intermediate(DType::I32, shape(&[queries, *classes])?)?;
				emitter.index_map(lookups, 0, 1)?;
				let counts = emitter.intermediate(DType::I32, shape(&[queries, *classes])?)?;
				emitter.gather(flat_counts, lookups, counts, 0)?;
				let sorted_counts = emitter.intermediate(DType::I32, shape(&[queries, *classes])?)?;
				let sorted_classes = emitter.intermediate(DType::I32, shape(&[queries, *classes])?)?;
				emitter.emit(
					vec![counts],
					vec![sorted_counts, sorted_classes],
					PrimitiveKind::Sort(Sort {
						axis: 1,
						direction: SortDirection::Descending,
						stable: true,
						emit_indices: true,
					}),
				)?;
				let first = emitter.intermediate(DType::I32, shape(&[1])?)?;
				emitter.index_map(first, 0, 1)?;
				emitter.gather(sorted_classes, first, predictions.id, 1)?;
			}
		}
	}
	emitter.finish()
}

pub fn append_knn_all_outputs(
	graph: &mut CalculationGraph,
	request: &KnnAllOutputRequest,
) -> OperationResult<KnnAllOutputMaterialization> {
	validate_boundary_storage(graph, request)?;
	let materialized = materialize_knn_all_outputs(request)?;
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
		return Err(knn_error(
			OperationErrorKind::IdentityNamespaceOverlap,
			format!("KNN intermediate {value} already exists in caller graph"),
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
		return Err(knn_error(
			OperationErrorKind::IdentityNamespaceOverlap,
			format!("KNN kernel {kernel} already exists in caller graph"),
		));
	}
	for prediction in request.outputs.iter().map(KnnOutputRequest::predictions) {
		if let Some(node) = graph
			.nodes
			.iter()
			.find(|node| node.kernel.outputs.contains(&prediction.id))
		{
			return Err(knn_error(
				OperationErrorKind::GraphMaterializationFailed,
				format!(
					"caller graph kernel {} already produces KNN output {}",
					node.kernel.id, prediction.id
				),
			));
		}
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

fn validate_request(request: &KnnAllOutputRequest) -> OperationResult<KnnAllOutputRequirements> {
	for tensor in [&request.query_features, &request.reference_features] {
		tensor.validate().map_err(graph_error)?;
		if tensor.dtype != DType::F32 || tensor.shape.rank() != 2 {
			return Err(knn_error(
				OperationErrorKind::UnsupportedConcreteShape,
				format!(
					"KNN feature tensor {} must be a rank-two f32 matrix",
					tensor.id
				),
			));
		}
	}
	let queries = request.query_features.shape.extents()[0];
	let dimensions = request.query_features.shape.extents()[1];
	let reference_rows = request.reference_features.shape.extents()[0];
	if request.reference_features.shape.extents()[1] != dimensions {
		return Err(knn_error(
			OperationErrorKind::UnsupportedConcreteShape,
			"KNN query and reference feature widths differ",
		));
	}
	let specs = request
		.outputs
		.iter()
		.map(KnnOutputRequest::spec)
		.collect::<Vec<_>>();
	let requirements = knn_all_output_requirements(
		queries,
		reference_rows,
		dimensions,
		request.neighbors,
		&specs,
	)?;
	let mut sources = BTreeSet::from([request.query_features.id, request.reference_features.id]);
	let mut predictions = BTreeSet::new();
	for (output_index, output) in request.outputs.iter().enumerate() {
		for tensor in [
			output.reference_values(),
			output.known(),
			output.predictions(),
		] {
			tensor.validate().map_err(graph_error)?;
		}
		let (value_dtype, prediction_dtype) = match output {
			KnnOutputRequest::Numeric { .. } => (DType::F32, DType::F32),
			KnnOutputRequest::Categorical { .. } => (DType::I32, DType::I32),
		};
		if output.reference_values().dtype != value_dtype
			|| output.reference_values().shape.extents() != [reference_rows]
		{
			return Err(knn_error(
				OperationErrorKind::UnsupportedConcreteShape,
				format!(
					"KNN output {output_index} reference values must be {value_dtype:?} [{reference_rows}]"
				),
			));
		}
		if output.known().dtype != DType::I32 || output.known().shape.extents() != [reference_rows] {
			return Err(knn_error(
				OperationErrorKind::UnsupportedConcreteShape,
				format!("KNN output {output_index} known mask must be I32 [{reference_rows}]"),
			));
		}
		if output.predictions().dtype != prediction_dtype || output.predictions().shape.extents() != [queries, 1] {
			return Err(knn_error(
				OperationErrorKind::UnsupportedConcreteShape,
				format!("KNN output {output_index} predictions must be {prediction_dtype:?} [{queries}, 1]"),
			));
		}
		sources.insert(output.reference_values().id);
		sources.insert(output.known().id);
		if sources.contains(&output.predictions().id) || !predictions.insert(output.predictions().id) {
			return Err(knn_error(
				OperationErrorKind::InvalidMaterializationRequest,
				format!("KNN output {output_index} prediction identity aliases another boundary"),
			));
		}
	}
	if request.tree_lanes == 0 || request.tree_lanes > MAXIMUM_TREE_LANES || !request.tree_lanes.is_power_of_two() {
		return Err(knn_error(
			OperationErrorKind::InvalidMaterializationRequest,
			format!("KNN reduction lanes must be a power of two in 1..={MAXIMUM_TREE_LANES}"),
		));
	}
	Ok(requirements)
}

fn validate_resources(request: &KnnAllOutputRequest, requirements: &KnnAllOutputRequirements) -> OperationResult<()> {
	if requirements.intermediate_values > request.identity_namespace.value_capacity() {
		return Err(knn_error(
			OperationErrorKind::IdentityNamespaceExhausted,
			format!(
				"KNN needs {} intermediate identities, namespace reserves {}",
				requirements.intermediate_values,
				request.identity_namespace.value_capacity()
			),
		));
	}
	if requirements.kernels > request.identity_namespace.kernel_capacity() {
		return Err(knn_error(
			OperationErrorKind::IdentityNamespaceExhausted,
			format!(
				"KNN needs {} kernel identities, namespace reserves {}",
				requirements.kernels,
				request.identity_namespace.kernel_capacity()
			),
		));
	}
	if requirements.workspace_bytes.get() > request.workspace_limit.get() {
		return Err(knn_error(
			OperationErrorKind::WorkspaceLimitExceeded,
			format!(
				"KNN needs {} workspace bytes, limit is {}",
				requirements.workspace_bytes.get(),
				request.workspace_limit.get()
			),
		));
	}
	Ok(())
}

struct KnnAllOutputEmitter {
	tensors: Vec<Tensor>,
	nodes: Vec<CalculationNode>,
	intermediate_values: Vec<ValueId>,
	kernels: Vec<KernelTemplateId>,
	next_value: u64,
	value_end: u64,
	next_kernel: u64,
	kernel_end: u64,
	workspace_bytes: u64,
	requirements: KnnAllOutputRequirements,
	identity_namespace: IdentityNamespace,
}

impl KnnAllOutputEmitter {
	fn new(request: &KnnAllOutputRequest, requirements: KnnAllOutputRequirements) -> OperationResult<Self> {
		let first_value = request.identity_namespace.first_value().get();
		let value_end = first_value
			.checked_add(request.identity_namespace.value_capacity())
			.ok_or_else(|| {
				knn_error(
					OperationErrorKind::IdentityNamespaceExhausted,
					"KNN value namespace overflows u64",
				)
			})?;
		let first_kernel = request.identity_namespace.first_kernel().get();
		let kernel_end = first_kernel
			.checked_add(request.identity_namespace.kernel_capacity())
			.ok_or_else(|| {
				knn_error(
					OperationErrorKind::IdentityNamespaceExhausted,
					"KNN kernel namespace overflows u64",
				)
			})?;
		let mut boundaries = BTreeMap::<ValueId, Tensor>::new();
		for source in request_source_tensors(request) {
			let mut tensor = source.clone();
			tensor.external_input = true;
			tensor.external_output = false;
			insert_boundary(&mut boundaries, tensor)?;
		}
		for prediction in request.outputs.iter().map(KnnOutputRequest::predictions) {
			if boundaries.contains_key(&prediction.id) {
				return Err(knn_error(
					OperationErrorKind::InvalidMaterializationRequest,
					format!("KNN prediction {} aliases an input tensor", prediction.id),
				));
			}
			let mut tensor = prediction.clone();
			tensor.external_input = false;
			tensor.external_output = true;
			insert_boundary(&mut boundaries, tensor)?;
		}
		if let Some(tensor) = boundaries
			.values()
			.find(|tensor| tensor.id.get() >= first_value && tensor.id.get() < value_end)
		{
			return Err(knn_error(
				OperationErrorKind::IdentityNamespaceOverlap,
				format!(
					"boundary tensor {} overlaps KNN intermediate range [{first_value}, {value_end})",
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
			return Err(knn_error(
				OperationErrorKind::IdentityNamespaceExhausted,
				"KNN intermediate identity range is exhausted",
			));
		}
		let value = ValueId::new(self.next_value);
		self.next_value += 1;
		let tensor = Tensor::contiguous(value, dtype, shape, false, false).map_err(graph_error)?;
		self.workspace_bytes = self
			.workspace_bytes
			.checked_add(tensor.storage_bytes.get())
			.ok_or_else(|| {
				knn_error(
					OperationErrorKind::WorkspaceArithmeticOverflow,
					"KNN workspace total overflows u64",
				)
			})?;
		self.intermediate_values.push(value);
		self.tensors.push(tensor);
		Ok(value)
	}

	fn emit(&mut self, inputs: Vec<ValueId>, outputs: Vec<ValueId>, kind: PrimitiveKind) -> OperationResult<()> {
		if self.next_kernel == self.kernel_end {
			return Err(knn_error(
				OperationErrorKind::IdentityNamespaceExhausted,
				"KNN kernel identity range is exhausted",
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

	fn reduce(
		&mut self,
		input: ValueId,
		output: ValueId,
		axes: &[usize],
		keep_dimensions: bool,
		tree_lanes: u32,
	) -> OperationResult<()> {
		self.emit(
			vec![input],
			vec![output],
			PrimitiveKind::Reduce(Reduce {
				operator: ReduceOperator::Sum,
				axes: AxisSet::new(axes.to_vec()).map_err(graph_error)?,
				keep_dimensions,
				result: ReduceResult::Value,
				tree_lanes,
			}),
		)
	}

	fn gather(&mut self, values: ValueId, indices: ValueId, output: ValueId, axis: usize) -> OperationResult<()> {
		self.emit(
			vec![values, indices],
			vec![output],
			PrimitiveKind::Gather(Gather {
				axis,
				bounds: IndexBounds::Reject,
			}),
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

	fn finish(self) -> OperationResult<KnnAllOutputMaterialization> {
		if self.intermediate_values.len() as u64 != self.requirements.intermediate_values
			|| self.kernels.len() as u64 != self.requirements.kernels
			|| self.workspace_bytes != self.requirements.workspace_bytes.get()
		{
			return Err(knn_error(
				OperationErrorKind::WorkspaceFormulaMismatch,
				format!(
					"KNN emitted {}/{}/{} values/kernels/bytes, expected {}/{}/{}",
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
		Ok(KnnAllOutputMaterialization {
			graph,
			intermediate_values: self.intermediate_values,
			kernels: self.kernels,
			output_requirements: self.requirements.outputs,
			workspace_bytes: ByteCount::new(self.workspace_bytes),
			identity_namespace: self.identity_namespace,
		})
	}
}

fn request_source_tensors(request: &KnnAllOutputRequest) -> Vec<&Tensor> {
	let mut tensors = vec![&request.query_features, &request.reference_features];
	for output in &request.outputs {
		tensors.extend([output.reference_values(), output.known()]);
	}
	tensors
}

fn validate_boundary_storage(graph: &CalculationGraph, request: &KnnAllOutputRequest) -> OperationResult<()> {
	let mut ids = BTreeSet::new();
	for tensor in &graph.tensors {
		if !ids.insert(tensor.id) {
			return Err(knn_error(
				OperationErrorKind::GraphMaterializationFailed,
				format!("caller graph repeats tensor {}", tensor.id),
			));
		}
	}
	for declared in request_source_tensors(request)
		.into_iter()
		.chain(request.outputs.iter().map(KnnOutputRequest::predictions))
	{
		let existing = graph
			.tensors
			.iter()
			.find(|tensor| tensor.id == declared.id)
			.ok_or_else(|| {
				knn_error(
					OperationErrorKind::InvalidMaterializationRequest,
					format!(
						"caller graph does not declare KNN boundary tensor {}",
						declared.id
					),
				)
			})?;
		if !same_storage_contract(existing, declared) {
			return Err(knn_error(
				OperationErrorKind::InvalidMaterializationRequest,
				format!(
					"caller graph tensor {} differs from the KNN storage contract",
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

fn insert_boundary(boundaries: &mut BTreeMap<ValueId, Tensor>, tensor: Tensor) -> OperationResult<()> {
	match boundaries.get(&tensor.id) {
		Some(existing) if same_storage_contract(existing, &tensor) => Ok(()),
		Some(_) => Err(knn_error(
			OperationErrorKind::InvalidMaterializationRequest,
			format!(
				"KNN boundary tensor {} has conflicting contracts",
				tensor.id
			),
		)),
		None => {
			boundaries.insert(tensor.id, tensor);
			Ok(())
		}
	}
}

fn square_program() -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder()?;
	let value = scalar_input(&mut builder, DType::F32)?;
	require_finite(&mut builder, value)?;
	let squared = scalar_binary(&mut builder, ScalarOpcode::Multiply, value, value)?;
	require_finite(&mut builder, squared)?;
	scalar_finish(builder, &[squared])
}

fn pairwise_l2_program() -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder()?;
	let query_norm = scalar_input(&mut builder, DType::F32)?;
	let reference_norm = scalar_input(&mut builder, DType::F32)?;
	let product = scalar_input(&mut builder, DType::F32)?;
	for value in [query_norm, reference_norm, product] {
		require_finite(&mut builder, value)?;
	}
	let two = builder.f32(2.0).map_err(graph_error)?;
	let twice_product = scalar_binary(&mut builder, ScalarOpcode::Multiply, product, two)?;
	let norm_sum = scalar_binary(&mut builder, ScalarOpcode::Add, query_norm, reference_norm)?;
	let squared_distance = scalar_binary(
		&mut builder,
		ScalarOpcode::Subtract,
		norm_sum,
		twice_product,
	)?;
	let zero = builder.f32(0.0).map_err(graph_error)?;
	let nonnegative = scalar_binary(&mut builder, ScalarOpcode::Maximum, squared_distance, zero)?;
	let distance = scalar_unary(&mut builder, ScalarOpcode::SquareRoot, nonnegative)?;
	require_finite(&mut builder, distance)?;
	scalar_finish(builder, &[distance])
}

fn known_count_program(expected: u64) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder()?;
	let count = scalar_input(&mut builder, DType::I32)?;
	let expected = builder
		.i32(i32::try_from(expected).map_err(|error| {
			knn_error(
				OperationErrorKind::UnsupportedConcreteShape,
				format!("KNN known count cannot be represented by i32: {error}"),
			)
		})?)
		.map_err(graph_error)?;
	let equal = scalar_binary(&mut builder, ScalarOpcode::Equal, count, expected)?;
	let _ = scalar_unary(&mut builder, ScalarOpcode::Require, equal)?;
	scalar_finish(builder, &[count])
}

fn masked_distance_program() -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder()?;
	let distance = scalar_input(&mut builder, DType::F32)?;
	let known = scalar_input(&mut builder, DType::I32)?;
	require_finite(&mut builder, distance)?;
	let zero = builder.i32(0).map_err(graph_error)?;
	let one = builder.i32(1).map_err(graph_error)?;
	let is_zero = scalar_binary(&mut builder, ScalarOpcode::Equal, known, zero)?;
	let is_one = scalar_binary(&mut builder, ScalarOpcode::Equal, known, one)?;
	let valid = scalar_binary(&mut builder, ScalarOpcode::BitOr, is_zero, is_one)?;
	let _ = scalar_unary(&mut builder, ScalarOpcode::Require, valid)?;
	let infinity = builder.f32(f32::INFINITY).map_err(graph_error)?;
	let selected = builder
		.ternary(ScalarOpcode::Select, known, distance, infinity)
		.map_err(graph_error)?;
	scalar_finish(builder, &[selected])
}

fn finite_divide_program(divisor: u64) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder()?;
	let value = scalar_input(&mut builder, DType::F32)?;
	require_finite(&mut builder, value)?;
	let divisor = builder.f32(divisor as f32).map_err(graph_error)?;
	let result = scalar_binary(&mut builder, ScalarOpcode::Divide, value, divisor)?;
	require_finite(&mut builder, result)?;
	scalar_finish(builder, &[result])
}

fn finite_identity_program() -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder()?;
	let value = scalar_input(&mut builder, DType::F32)?;
	require_finite(&mut builder, value)?;
	scalar_finish(builder, &[value])
}

fn categorical_code_program(classes: u64) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder()?;
	let code = scalar_input(&mut builder, DType::I32)?;
	require_categorical_code(&mut builder, code, classes)?;
	scalar_finish(builder, &[code])
}

fn categorical_bin_program(classes: u64) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder()?;
	let row_base = scalar_input(&mut builder, DType::I32)?;
	let code = scalar_input(&mut builder, DType::I32)?;
	require_categorical_code(&mut builder, code, classes)?;
	let bin = scalar_binary(&mut builder, ScalarOpcode::Add, row_base, code)?;
	scalar_finish(builder, &[bin])
}

fn require_categorical_code(
	builder: &mut ScalarProgramBuilder,
	code: ScalarExpression,
	classes: u64,
) -> OperationResult<()> {
	let zero = builder.i32(0).map_err(graph_error)?;
	let classes = builder
		.i32(i32::try_from(classes).map_err(|error| {
			knn_error(
				OperationErrorKind::UnsupportedConcreteShape,
				format!("KNN class count cannot be represented by i32: {error}"),
			)
		})?)
		.map_err(graph_error)?;
	let lower = scalar_binary(builder, ScalarOpcode::GreaterThanOrEqual, code, zero)?;
	let upper = scalar_binary(builder, ScalarOpcode::LessThan, code, classes)?;
	let valid = scalar_binary(builder, ScalarOpcode::BitAnd, lower, upper)?;
	let _ = scalar_unary(builder, ScalarOpcode::Require, valid)?;
	Ok(())
}

fn require_finite(builder: &mut ScalarProgramBuilder, value: ScalarExpression) -> OperationResult<()> {
	let finite = scalar_unary(builder, ScalarOpcode::IsFinite, value)?;
	let _ = scalar_unary(builder, ScalarOpcode::Require, finite)?;
	Ok(())
}

fn scalar_builder() -> OperationResult<ScalarProgramBuilder> {
	ScalarProgramBuilder::new().map_err(graph_error)
}

fn scalar_input(builder: &mut ScalarProgramBuilder, dtype: DType) -> OperationResult<ScalarExpression> {
	builder.input(dtype).map_err(graph_error)
}

fn scalar_unary(
	builder: &mut ScalarProgramBuilder,
	opcode: ScalarOpcode,
	value: ScalarExpression,
) -> OperationResult<ScalarExpression> {
	builder.unary(opcode, value).map_err(graph_error)
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
			knn_error(
				OperationErrorKind::WorkspaceArithmeticOverflow,
				format!("{role} overflowed u64"),
			)
		})
	})
}

fn checked_sum(values: &[u64], role: &str) -> OperationResult<u64> {
	values.iter().copied().try_fold(0_u64, |sum, value| {
		sum.checked_add(value).ok_or_else(|| {
			knn_error(
				OperationErrorKind::WorkspaceArithmeticOverflow,
				format!("{role} overflowed u64"),
			)
		})
	})
}

fn graph_error(error: impl ToString) -> OperationError {
	knn_error(
		OperationErrorKind::GraphMaterializationFailed,
		error.to_string(),
	)
}

fn knn_error(kind: OperationErrorKind, detail: impl Into<String>) -> OperationError {
	OperationError::new(kind, detail)
}
