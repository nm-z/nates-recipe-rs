use std::collections::BTreeMap;

use recipe_core::{AliasPermission, ByteCount, DType, KernelTemplateId, ScalarOpcode, ValueId};
use recipe_language::{
	AxisSet, CalculationGraph, CalculationNode, Contraction, Elementwise, Gather, IndexBounds, IndexMap,
	PrimitiveAliasRule, PrimitiveKernel, PrimitiveKind, Reduce, ReduceOperator, ReduceResult, ScalarExpression,
	ScalarProgramBuilder, Shape, Tensor,
};

use crate::{IdentityNamespace, OperationError, OperationErrorKind, OperationResult};

const INITIALIZATION_INTERMEDIATES: u64 = 1;
const INITIALIZATION_KERNELS: u64 = 2;
const LLOYD_INTERMEDIATES: u64 = 16;
const LLOYD_KERNELS: u64 = 18;
const MAXIMUM_TREE_LANES: u32 = 1_024;
const MAXIMUM_EXACT_F32_COUNT: u64 = 16_777_216;

/// Deterministic first-state construction for a K-means block.
///
/// Centroid `c` starts from training row `c mod rows`. Thus the first
/// `min(clusters, rows)` centroids preserve source-row order, while an explicit
/// cluster count larger than the population cycles deterministically.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KMeansInitializationRequest {
	pub points: Tensor,
	pub centroids: Tensor,
	pub identity_namespace: IdentityNamespace,
	pub workspace_limit: ByteCount,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KMeansInitializationMaterialization {
	pub graph: CalculationGraph,
	pub intermediate_values: Vec<ValueId>,
	pub kernels: Vec<KernelTemplateId>,
	pub workspace_bytes: ByteCount,
	pub identity_namespace: IdentityNamespace,
}

/// One deterministic Lloyd state transition.
///
/// Assignment uses rooted L2 and the fixed reduction tree's lowest-index tie
/// rule. Nonempty centroids become the mean of every assigned row. An empty
/// centroid retains its prior value. `distances` is recomputed against the
/// updated centroids and contains one distance per centroid for every row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KMeansLloydRequest {
	pub points: Tensor,
	pub prior_centroids: Tensor,
	pub updated_centroids: Tensor,
	pub distances: Tensor,
	pub tree_lanes: u32,
	pub identity_namespace: IdentityNamespace,
	pub workspace_limit: ByteCount,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KMeansRequirements {
	pub intermediate_values: u64,
	pub kernels: u64,
	pub workspace_bytes: ByteCount,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KMeansLloydMaterialization {
	pub graph: CalculationGraph,
	pub intermediate_values: Vec<ValueId>,
	pub kernels: Vec<KernelTemplateId>,
	pub centroid_update_kernel: KernelTemplateId,
	pub workspace_bytes: ByteCount,
	pub identity_namespace: IdentityNamespace,
}

pub fn kmeans_initialization_requirements(clusters: u64) -> OperationResult<KMeansRequirements> {
	if clusters == 0 || clusters > i32::MAX as u64 {
		return Err(kmeans_error(
			OperationErrorKind::UnsupportedConcreteShape,
			"K-means cluster count must be in 1..=i32::MAX",
		));
	}
	Ok(KMeansRequirements {
		intermediate_values: INITIALIZATION_INTERMEDIATES,
		kernels: INITIALIZATION_KERNELS,
		workspace_bytes: ByteCount::new(checked_product(
			&[clusters, 4],
			"K-means initialization workspace",
		)?),
	})
}

pub fn kmeans_lloyd_requirements(rows: u64, dimensions: u64, clusters: u64) -> OperationResult<KMeansRequirements> {
	validate_dimensions(rows, dimensions, clusters)?;
	let row_features = checked_product(&[rows, dimensions], "K-means row-feature elements")?;
	let centroid_features = checked_product(&[clusters, dimensions], "K-means centroid-feature elements")?;
	let assignments = checked_product(&[rows, clusters], "K-means assignment elements")?;
	let workspace_elements = checked_sum(
		&[
			checked_product(&[3, row_features], "K-means row-feature workspace")?,
			checked_product(
				&[4, centroid_features],
				"K-means centroid-feature workspace",
			)?,
			checked_product(&[2, rows], "K-means row workspace")?,
			checked_product(&[3, clusters], "K-means cluster workspace")?,
			checked_product(&[4, assignments], "K-means assignment workspace")?,
		],
		"K-means workspace elements",
	)?;
	Ok(KMeansRequirements {
		intermediate_values: LLOYD_INTERMEDIATES,
		kernels: LLOYD_KERNELS,
		workspace_bytes: ByteCount::new(checked_product(
			&[workspace_elements, 4],
			"K-means workspace bytes",
		)?),
	})
}

pub fn materialize_kmeans_initialization(
	request: &KMeansInitializationRequest,
) -> OperationResult<KMeansInitializationMaterialization> {
	validate_f32_matrix("points", &request.points)?;
	validate_f32_matrix("centroids", &request.centroids)?;
	let rows = request.points.shape.extents()[0];
	let dimensions = request.points.shape.extents()[1];
	let clusters = request.centroids.shape.extents()[0];
	if request.centroids.shape.extents()[1] != dimensions {
		return Err(kmeans_error(
			OperationErrorKind::UnsupportedConcreteShape,
			"K-means centroid width differs from point width",
		));
	}
	validate_dimensions(rows, dimensions, clusters)?;
	let requirements = kmeans_initialization_requirements(clusters)?;
	validate_resources(
		request.identity_namespace,
		request.workspace_limit,
		requirements,
		"K-means initialization",
	)?;
	let mut emitter = KMeansEmitter::new(
		[
			(&request.points, true, false),
			(&request.centroids, false, true),
		],
		request.identity_namespace,
		"K-means initialization",
	)?;
	let indices = emitter.intermediate(DType::I32, shape(&[clusters])?)?;
	emitter.emit(
		Vec::new(),
		vec![indices],
		PrimitiveKind::IndexMap(IndexMap {
			start: 0,
			element_step: 1,
			iteration_step: 0,
			modulus: Some(checked_i32(rows, "K-means initialization row count")?),
		}),
	)?;
	emitter.emit(
		vec![request.points.id, indices],
		vec![request.centroids.id],
		checked_gather(0),
	)?;
	let parts = emitter.finish(requirements)?;
	Ok(KMeansInitializationMaterialization {
		graph: parts.graph,
		intermediate_values: parts.intermediate_values,
		kernels: parts.kernels,
		workspace_bytes: parts.workspace_bytes,
		identity_namespace: request.identity_namespace,
	})
}

pub fn materialize_kmeans_lloyd_step(request: &KMeansLloydRequest) -> OperationResult<KMeansLloydMaterialization> {
	for (name, tensor) in [
		("points", &request.points),
		("prior centroids", &request.prior_centroids),
		("updated centroids", &request.updated_centroids),
		("distances", &request.distances),
	] {
		validate_f32_matrix(name, tensor)?;
	}
	let rows = request.points.shape.extents()[0];
	let dimensions = request.points.shape.extents()[1];
	let clusters = request.prior_centroids.shape.extents()[0];
	validate_dimensions(rows, dimensions, clusters)?;
	if request.prior_centroids.shape.extents()[1] != dimensions
		|| request.updated_centroids.shape.extents() != [clusters, dimensions]
	{
		return Err(kmeans_error(
			OperationErrorKind::UnsupportedConcreteShape,
			"K-means prior and updated centroids must both be [clusters, point dimensions]",
		));
	}
	if request.distances.shape.extents() != [rows, clusters] {
		return Err(kmeans_error(
			OperationErrorKind::UnsupportedConcreteShape,
			"K-means distances must be [rows, clusters]",
		));
	}
	if request.tree_lanes == 0 || request.tree_lanes > MAXIMUM_TREE_LANES || !request.tree_lanes.is_power_of_two() {
		return Err(kmeans_error(
			OperationErrorKind::InvalidMaterializationRequest,
			format!("K-means tree lanes must be a power of two in 1..={MAXIMUM_TREE_LANES}"),
		));
	}
	if request.updated_centroids.id == request.points.id
		|| request.updated_centroids.id == request.prior_centroids.id
		|| request.updated_centroids.id == request.distances.id
		|| request.distances.id == request.points.id
		|| request.distances.id == request.prior_centroids.id
	{
		return Err(kmeans_error(
			OperationErrorKind::InvalidMaterializationRequest,
			"K-means output tensor identities must be distinct from every input and each other",
		));
	}
	let requirements = kmeans_lloyd_requirements(rows, dimensions, clusters)?;
	validate_resources(
		request.identity_namespace,
		request.workspace_limit,
		requirements,
		"K-means Lloyd step",
	)?;
	let mut emitter = KMeansEmitter::new(
		[
			(&request.points, true, false),
			(&request.prior_centroids, true, false),
			(&request.updated_centroids, false, true),
			(&request.distances, false, true),
		],
		request.identity_namespace,
		"K-means Lloyd step",
	)?;

	let point_squared = emitter.intermediate(DType::F32, shape(&[rows, dimensions])?)?;
	let centroid_squared = emitter.intermediate(DType::F32, shape(&[clusters, dimensions])?)?;
	emitter.emit(
		vec![request.points.id],
		vec![point_squared],
		elementwise(square_program()?),
	)?;
	emitter.emit(
		vec![request.prior_centroids.id],
		vec![centroid_squared],
		elementwise(square_program()?),
	)?;
	let point_norms = emitter.intermediate(DType::F32, shape(&[rows, 1])?)?;
	let centroid_norms = emitter.intermediate(DType::F32, shape(&[clusters])?)?;
	emitter.emit(
		vec![point_squared],
		vec![point_norms],
		sum_reduction(&[1], true, request.tree_lanes)?,
	)?;
	emitter.emit(
		vec![centroid_squared],
		vec![centroid_norms],
		sum_reduction(&[1], false, request.tree_lanes)?,
	)?;
	let distance_shape = shape(&[rows, clusters])?;
	let products = emitter.intermediate(DType::F32, distance_shape.clone())?;
	emitter.emit(
		vec![request.points.id, request.prior_centroids.id],
		vec![products],
		matrix_row_products(),
	)?;
	let prior_distances = emitter.intermediate(DType::F32, distance_shape.clone())?;
	emitter.emit(
		vec![point_norms, centroid_norms, products],
		vec![prior_distances],
		elementwise(pairwise_l2_program()?),
	)?;

	let assignments = emitter.intermediate(DType::I32, shape(&[rows, 1])?)?;
	emitter.emit(
		vec![prior_distances],
		vec![assignments],
		PrimitiveKind::Reduce(Reduce {
			operator: ReduceOperator::Minimum,
			axes: AxisSet::new(vec![1]).map_err(graph_error)?,
			keep_dimensions: true,
			result: ReduceResult::Index,
			tree_lanes: request.tree_lanes,
		}),
	)?;
	let cluster_indices = emitter.intermediate(DType::I32, shape(&[clusters])?)?;
	emitter.emit(
		Vec::new(),
		vec![cluster_indices],
		PrimitiveKind::IndexMap(IndexMap {
			start: 0,
			element_step: 1,
			iteration_step: 0,
			modulus: None,
		}),
	)?;
	let memberships = emitter.intermediate(DType::F32, distance_shape)?;
	emitter.emit(
		vec![assignments, cluster_indices],
		vec![memberships],
		elementwise(membership_program()?),
	)?;

	let point_shape = shape(&[rows, dimensions])?;
	let one_seed = emitter.intermediate(DType::I32, point_shape.clone())?;
	emitter.emit(
		Vec::new(),
		vec![one_seed],
		PrimitiveKind::IndexMap(IndexMap {
			start: 0,
			element_step: 0,
			iteration_step: 0,
			modulus: None,
		}),
	)?;
	let ones = emitter.intermediate(DType::F32, point_shape)?;
	emitter.emit(vec![one_seed], vec![ones], elementwise(one_program()?))?;
	let centroid_shape = shape(&[clusters, dimensions])?;
	let sums = emitter.intermediate(DType::F32, centroid_shape.clone())?;
	let counts = emitter.intermediate(DType::F32, centroid_shape.clone())?;
	emitter.emit(
		vec![memberships, request.points.id],
		vec![sums],
		membership_contraction(),
	)?;
	emitter.emit(
		vec![memberships, ones],
		vec![counts],
		membership_contraction(),
	)?;
	let centroid_update_kernel = emitter.emit_with_aliases(
		vec![sums, counts, request.prior_centroids.id],
		vec![request.updated_centroids.id],
		elementwise(centroid_update_program()?),
		vec![
			PrimitiveAliasRule {
				input: 0,
				output: 0,
				permission: AliasPermission::Forbidden,
			},
			PrimitiveAliasRule {
				input: 1,
				output: 0,
				permission: AliasPermission::Forbidden,
			},
			PrimitiveAliasRule {
				input: 2,
				output: 0,
				permission: AliasPermission::MustAliasExact,
			},
		],
	)?;

	let updated_squared = emitter.intermediate(DType::F32, centroid_shape)?;
	emitter.emit(
		vec![request.updated_centroids.id],
		vec![updated_squared],
		elementwise(square_program()?),
	)?;
	let updated_norms = emitter.intermediate(DType::F32, shape(&[clusters])?)?;
	emitter.emit(
		vec![updated_squared],
		vec![updated_norms],
		sum_reduction(&[1], false, request.tree_lanes)?,
	)?;
	let updated_products = emitter.intermediate(DType::F32, shape(&[rows, clusters])?)?;
	emitter.emit(
		vec![request.points.id, request.updated_centroids.id],
		vec![updated_products],
		matrix_row_products(),
	)?;
	emitter.emit(
		vec![point_norms, updated_norms, updated_products],
		vec![request.distances.id],
		elementwise(pairwise_l2_program()?),
	)?;
	let parts = emitter.finish(requirements)?;
	Ok(KMeansLloydMaterialization {
		graph: parts.graph,
		intermediate_values: parts.intermediate_values,
		kernels: parts.kernels,
		centroid_update_kernel,
		workspace_bytes: parts.workspace_bytes,
		identity_namespace: request.identity_namespace,
	})
}

struct KMeansParts {
	graph: CalculationGraph,
	intermediate_values: Vec<ValueId>,
	kernels: Vec<KernelTemplateId>,
	workspace_bytes: ByteCount,
}

struct KMeansEmitter {
	tensors: Vec<Tensor>,
	nodes: Vec<CalculationNode>,
	intermediate_values: Vec<ValueId>,
	kernels: Vec<KernelTemplateId>,
	next_value: u64,
	value_end: u64,
	next_kernel: u64,
	kernel_end: u64,
	workspace_bytes: u64,
	context: &'static str,
}

impl KMeansEmitter {
	fn new<'a>(
		boundaries: impl IntoIterator<Item = (&'a Tensor, bool, bool)>,
		namespace: IdentityNamespace,
		context: &'static str,
	) -> OperationResult<Self> {
		let first_value = namespace.first_value().get();
		let value_end = first_value
			.checked_add(namespace.value_capacity())
			.ok_or_else(|| {
				kmeans_error(
					OperationErrorKind::IdentityNamespaceExhausted,
					format!("{context} value namespace overflows u64"),
				)
			})?;
		let first_kernel = namespace.first_kernel().get();
		let kernel_end = first_kernel
			.checked_add(namespace.kernel_capacity())
			.ok_or_else(|| {
				kmeans_error(
					OperationErrorKind::IdentityNamespaceExhausted,
					format!("{context} kernel namespace overflows u64"),
				)
			})?;
		let mut tensors = BTreeMap::<ValueId, Tensor>::new();
		for (boundary, input, output) in boundaries {
			boundary.validate().map_err(|error| {
				kmeans_error(
					OperationErrorKind::InvalidMaterializationRequest,
					error.to_string(),
				)
			})?;
			let mut tensor = boundary.clone();
			tensor.external_input = input;
			tensor.external_output = output;
			if tensors.insert(tensor.id, tensor).is_some() {
				return Err(kmeans_error(
					OperationErrorKind::InvalidMaterializationRequest,
					format!("{context} repeats a boundary tensor identity"),
				));
			}
		}
		if let Some(tensor) = tensors
			.values()
			.find(|tensor| tensor.id.get() >= first_value && tensor.id.get() < value_end)
		{
			return Err(kmeans_error(
				OperationErrorKind::IdentityNamespaceOverlap,
				format!(
					"boundary tensor {} overlaps {context} intermediate range [{first_value}, {value_end})",
					tensor.id
				),
			));
		}
		Ok(Self {
			tensors: tensors.into_values().collect(),
			nodes: Vec::new(),
			intermediate_values: Vec::new(),
			kernels: Vec::new(),
			next_value: first_value,
			value_end,
			next_kernel: first_kernel,
			kernel_end,
			workspace_bytes: 0,
			context,
		})
	}

	fn intermediate(&mut self, dtype: DType, shape: Shape) -> OperationResult<ValueId> {
		if self.next_value == self.value_end {
			return Err(kmeans_error(
				OperationErrorKind::IdentityNamespaceExhausted,
				format!("{} intermediate identity range is exhausted", self.context),
			));
		}
		let value = ValueId::new(self.next_value);
		self.next_value += 1;
		let tensor = Tensor::contiguous(value, dtype, shape, false, false).map_err(graph_error)?;
		self.workspace_bytes = self
			.workspace_bytes
			.checked_add(tensor.storage_bytes.get())
			.ok_or_else(|| {
				kmeans_error(
					OperationErrorKind::WorkspaceArithmeticOverflow,
					format!("{} workspace overflows u64", self.context),
				)
			})?;
		self.intermediate_values.push(value);
		self.tensors.push(tensor);
		Ok(value)
	}

	fn emit(&mut self, inputs: Vec<ValueId>, outputs: Vec<ValueId>, kind: PrimitiveKind) -> OperationResult<()> {
		let aliases = forbidden_aliases(inputs.len(), outputs.len());
		self.emit_with_aliases(inputs, outputs, kind, aliases)
			.map(|_| ())
	}

	fn emit_with_aliases(
		&mut self,
		inputs: Vec<ValueId>,
		outputs: Vec<ValueId>,
		kind: PrimitiveKind,
		alias_rules: Vec<PrimitiveAliasRule>,
	) -> OperationResult<KernelTemplateId> {
		if self.next_kernel == self.kernel_end {
			return Err(kmeans_error(
				OperationErrorKind::IdentityNamespaceExhausted,
				format!("{} kernel identity range is exhausted", self.context),
			));
		}
		let id = KernelTemplateId::new(self.next_kernel);
		self.next_kernel += 1;
		self.kernels.push(id);
		self.nodes.push(CalculationNode {
			kernel: PrimitiveKernel {
				id,
				inputs,
				outputs,
				alias_rules,
				kind,
			},
		});
		Ok(id)
	}

	fn finish(self, requirements: KMeansRequirements) -> OperationResult<KMeansParts> {
		if self.intermediate_values.len() as u64 != requirements.intermediate_values
			|| self.kernels.len() as u64 != requirements.kernels
		{
			return Err(kmeans_error(
				OperationErrorKind::GraphMaterializationFailed,
				format!(
					"{} emitted {}/{} intermediates and {}/{} kernels",
					self.context,
					self.intermediate_values.len(),
					requirements.intermediate_values,
					self.kernels.len(),
					requirements.kernels
				),
			));
		}
		if self.workspace_bytes != requirements.workspace_bytes.get() {
			return Err(kmeans_error(
				OperationErrorKind::WorkspaceFormulaMismatch,
				format!(
					"{} emitted {} workspace bytes, formula requires {}",
					self.context,
					self.workspace_bytes,
					requirements.workspace_bytes.get()
				),
			));
		}
		let graph = CalculationGraph {
			tensors: self.tensors,
			nodes: self.nodes,
		};
		graph.validate().map_err(graph_error)?;
		Ok(KMeansParts {
			graph,
			intermediate_values: self.intermediate_values,
			kernels: self.kernels,
			workspace_bytes: ByteCount::new(self.workspace_bytes),
		})
	}
}

fn validate_f32_matrix(name: &str, tensor: &Tensor) -> OperationResult<()> {
	tensor.validate().map_err(|error| {
		kmeans_error(
			OperationErrorKind::InvalidMaterializationRequest,
			error.to_string(),
		)
	})?;
	if tensor.dtype != DType::F32 || tensor.shape.rank() != 2 {
		return Err(kmeans_error(
			OperationErrorKind::InvalidMaterializationRequest,
			format!("K-means {name} must be a rank-two f32 matrix"),
		));
	}
	Ok(())
}

fn validate_dimensions(rows: u64, dimensions: u64, clusters: u64) -> OperationResult<()> {
	if rows == 0 || rows > MAXIMUM_EXACT_F32_COUNT {
		return Err(kmeans_error(
			OperationErrorKind::UnsupportedConcreteShape,
			format!("K-means row count must be in 1..={MAXIMUM_EXACT_F32_COUNT}"),
		));
	}
	if dimensions == 0 {
		return Err(kmeans_error(
			OperationErrorKind::UnsupportedConcreteShape,
			"K-means feature width must be nonzero",
		));
	}
	if clusters == 0 || clusters > i32::MAX as u64 {
		return Err(kmeans_error(
			OperationErrorKind::UnsupportedConcreteShape,
			"K-means cluster count must be in 1..=i32::MAX",
		));
	}
	Ok(())
}

fn validate_resources(
	namespace: IdentityNamespace,
	workspace_limit: ByteCount,
	requirements: KMeansRequirements,
	context: &str,
) -> OperationResult<()> {
	if requirements.intermediate_values > namespace.value_capacity() {
		return Err(kmeans_error(
			OperationErrorKind::IdentityNamespaceExhausted,
			format!(
				"{context} needs {} values, namespace reserves {}",
				requirements.intermediate_values,
				namespace.value_capacity()
			),
		));
	}
	if requirements.kernels > namespace.kernel_capacity() {
		return Err(kmeans_error(
			OperationErrorKind::IdentityNamespaceExhausted,
			format!(
				"{context} needs {} kernels, namespace reserves {}",
				requirements.kernels,
				namespace.kernel_capacity()
			),
		));
	}
	if requirements.workspace_bytes.get() > workspace_limit.get() {
		return Err(kmeans_error(
			OperationErrorKind::WorkspaceLimitExceeded,
			format!(
				"{context} needs {} workspace bytes, limit is {}",
				requirements.workspace_bytes.get(),
				workspace_limit.get()
			),
		));
	}
	Ok(())
}

fn shape(extents: &[u64]) -> OperationResult<Shape> { Shape::new(extents.to_vec()).map_err(graph_error) }

fn checked_i32(value: u64, context: &str) -> OperationResult<i32> {
	i32::try_from(value).map_err(|error| {
		kmeans_error(
			OperationErrorKind::UnsupportedConcreteShape,
			format!("{context} cannot be represented by i32: {error}"),
		)
	})
}

fn elementwise(program: recipe_core::ScalarProgram) -> PrimitiveKind {
	PrimitiveKind::Elementwise(Elementwise { program })
}

fn matrix_row_products() -> PrimitiveKind {
	PrimitiveKind::Contraction(Contraction {
		batch_axes: Vec::new(),
		contract_axes: vec![(1, 1)],
	})
}

fn membership_contraction() -> PrimitiveKind {
	PrimitiveKind::Contraction(Contraction {
		batch_axes: Vec::new(),
		contract_axes: vec![(0, 0)],
	})
}

fn sum_reduction(axes: &[usize], keep_dimensions: bool, tree_lanes: u32) -> OperationResult<PrimitiveKind> {
	Ok(PrimitiveKind::Reduce(Reduce {
		operator: ReduceOperator::Sum,
		axes: AxisSet::new(axes.to_vec()).map_err(graph_error)?,
		keep_dimensions,
		result: ReduceResult::Value,
		tree_lanes,
	}))
}

fn checked_gather(axis: usize) -> PrimitiveKind {
	PrimitiveKind::Gather(Gather {
		axis,
		bounds: IndexBounds::Reject,
	})
}

fn square_program() -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder()?;
	let value = scalar_input(&mut builder, DType::F32)?;
	let squared = scalar_binary(&mut builder, ScalarOpcode::Multiply, value, value)?;
	scalar_finish(builder, &[squared])
}

fn pairwise_l2_program() -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder()?;
	let point_norm = scalar_input(&mut builder, DType::F32)?;
	let centroid_norm = scalar_input(&mut builder, DType::F32)?;
	let product = scalar_input(&mut builder, DType::F32)?;
	let two = scalar_f32(&mut builder, 2.0)?;
	let twice_product = scalar_binary(&mut builder, ScalarOpcode::Multiply, product, two)?;
	let norm_sum = scalar_binary(&mut builder, ScalarOpcode::Add, point_norm, centroid_norm)?;
	let squared_distance = scalar_binary(
		&mut builder,
		ScalarOpcode::Subtract,
		norm_sum,
		twice_product,
	)?;
	let zero = scalar_f32(&mut builder, 0.0)?;
	let nonnegative = scalar_binary(&mut builder, ScalarOpcode::Maximum, squared_distance, zero)?;
	let distance = scalar_unary(&mut builder, ScalarOpcode::SquareRoot, nonnegative)?;
	scalar_finish(builder, &[distance])
}

fn membership_program() -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder()?;
	let assignment = scalar_input(&mut builder, DType::I32)?;
	let cluster = scalar_input(&mut builder, DType::I32)?;
	let member = scalar_binary(&mut builder, ScalarOpcode::Equal, assignment, cluster)?;
	let member = scalar_unary(&mut builder, ScalarOpcode::ConvertI32ToF32, member)?;
	scalar_finish(builder, &[member])
}

fn one_program() -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder()?;
	let _seed = scalar_input(&mut builder, DType::I32)?;
	let one = scalar_f32(&mut builder, 1.0)?;
	scalar_finish(builder, &[one])
}

fn centroid_update_program() -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder()?;
	let sum = scalar_input(&mut builder, DType::F32)?;
	let count = scalar_input(&mut builder, DType::F32)?;
	let prior = scalar_input(&mut builder, DType::F32)?;
	let zero = scalar_f32(&mut builder, 0.0)?;
	let one = scalar_f32(&mut builder, 1.0)?;
	let populated = scalar_binary(&mut builder, ScalarOpcode::GreaterThan, count, zero)?;
	let safe_count = scalar_binary(&mut builder, ScalarOpcode::Maximum, count, one)?;
	let mean = scalar_binary(&mut builder, ScalarOpcode::Divide, sum, safe_count)?;
	let centroid = scalar_ternary(&mut builder, ScalarOpcode::Select, populated, mean, prior)?;
	scalar_finish(builder, &[centroid])
}

fn scalar_builder() -> OperationResult<ScalarProgramBuilder> { ScalarProgramBuilder::new().map_err(graph_error) }

fn scalar_input(builder: &mut ScalarProgramBuilder, dtype: DType) -> OperationResult<ScalarExpression> {
	builder.input(dtype).map_err(graph_error)
}

fn scalar_f32(builder: &mut ScalarProgramBuilder, value: f32) -> OperationResult<ScalarExpression> {
	builder.f32(value).map_err(graph_error)
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

fn scalar_ternary(
	builder: &mut ScalarProgramBuilder,
	opcode: ScalarOpcode,
	first: ScalarExpression,
	second: ScalarExpression,
	third: ScalarExpression,
) -> OperationResult<ScalarExpression> {
	builder
		.ternary(opcode, first, second, third)
		.map_err(graph_error)
}

fn scalar_finish(
	builder: ScalarProgramBuilder,
	outputs: &[ScalarExpression],
) -> OperationResult<recipe_core::ScalarProgram> {
	builder.finish(outputs).map_err(graph_error)
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

fn checked_product(values: &[u64], context: &str) -> OperationResult<u64> {
	values.iter().try_fold(1_u64, |product, value| {
		product.checked_mul(*value).ok_or_else(|| {
			kmeans_error(
				OperationErrorKind::WorkspaceArithmeticOverflow,
				format!("{context} overflows u64"),
			)
		})
	})
}

fn checked_sum(values: &[u64], context: &str) -> OperationResult<u64> {
	values.iter().try_fold(0_u64, |sum, value| {
		sum.checked_add(*value).ok_or_else(|| {
			kmeans_error(
				OperationErrorKind::WorkspaceArithmeticOverflow,
				format!("{context} overflows u64"),
			)
		})
	})
}

fn graph_error(error: impl ToString) -> OperationError {
	kmeans_error(
		OperationErrorKind::GraphMaterializationFailed,
		error.to_string(),
	)
}

fn kmeans_error(kind: OperationErrorKind, detail: impl Into<String>) -> OperationError {
	OperationError::new(kind, detail)
}
