use std::collections::{BTreeMap, BTreeSet};

use recipe_core::{AliasPermission, ByteCount, DType, KernelTemplateId, ScalarOpcode, ValueId};
use recipe_language::{
	AxisSet, CalculationGraph, CalculationNode, Elementwise, Gather, IndexBounds, PrimitiveAliasRule,
	PrimitiveKernel, PrimitiveKind, Reduce, ReduceOperator, ReduceResult, ScalarProgramBuilder, Scatter,
	ScatterConflict, Shape, Tensor,
};

use crate::{
	CompositionStep, IterationBound, LoweringAvailability, OperationDescriptor, OperationError, OperationErrorKind,
	OperationId, OperationResult, PrimitiveFamily, operation_registry,
};

mod attention_sequence_embedding;
mod convolution_pooling;
mod creation_shape_misc;
mod graph_cluster_rl;
mod indexing_sort_encoding;
mod inference_quantization_diffusion;
mod loss_metrics;
mod optimizer_normalization;
mod solver_fft;
mod tree_boosting;

const MAX_EXPANDED_STEPS: usize = 1_000_000;
const MAX_I32_INDEX: u64 = 2_147_483_647;
const MISSING_COMPONENTS: &[MissingConcreteComponent] = &[
	MissingConcreteComponent::TensorAbi,
	MissingConcreteComponent::ScalarFormula,
	MissingConcreteComponent::PrimitiveParameters,
	MissingConcreteComponent::WorkspacePolicy,
];

/// One immutable, name-addressed tensor declaration at the composition
/// preparation boundary.
#[derive(Clone, Copy, Debug)]
pub struct NamedTensor<'a> {
	name: &'a str,
	tensor: &'a Tensor,
}

impl<'a> NamedTensor<'a> {
	#[must_use]
	pub const fn new(name: &'a str, tensor: &'a Tensor) -> Self {
		Self { name, tensor }
	}

	#[must_use]
	pub const fn name(self) -> &'a str {
		self.name
	}

	#[must_use]
	pub const fn tensor(self) -> &'a Tensor {
		self.tensor
	}
}

/// Typed values fixed before the static calculation graph is built.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreparedParameter {
	U64(u64),
	I32(i32),
	F32Bits(u32),
	Bool(bool),
}

pub type PreparedParameters = BTreeMap<String, PreparedParameter>;

/// Caller-reserved identities for intermediates and kernels emitted by one
/// materialized fragment.
///
/// The ranges are half-open: `first_* .. first_* + *_capacity`. Input and
/// output tensor identities are supplied by the caller separately and must not
/// fall inside the reserved intermediate-value range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IdentityNamespace {
	first_value: ValueId,
	value_capacity: u64,
	first_kernel: KernelTemplateId,
	kernel_capacity: u64,
}

impl IdentityNamespace {
	#[must_use]
	pub const fn new(
		first_value: ValueId,
		value_capacity: u64,
		first_kernel: KernelTemplateId,
		kernel_capacity: u64,
	) -> Self {
		Self {
			first_value,
			value_capacity,
			first_kernel,
			kernel_capacity,
		}
	}

	#[must_use]
	pub const fn first_value(self) -> ValueId {
		self.first_value
	}

	#[must_use]
	pub const fn value_capacity(self) -> u64 {
		self.value_capacity
	}

	#[must_use]
	pub const fn first_kernel(self) -> KernelTemplateId {
		self.first_kernel
	}

	#[must_use]
	pub const fn kernel_capacity(self) -> u64 {
		self.kernel_capacity
	}
}

/// Immutable request used to turn a structured operation into a concrete
/// recipe-language graph.
#[derive(Clone, Copy, Debug)]
pub struct MaterializationRequest<'a> {
	descriptor: OperationDescriptor,
	inputs: &'a [NamedTensor<'a>],
	outputs: &'a [NamedTensor<'a>],
	iteration_shape_input: &'a str,
	parameters: &'a PreparedParameters,
	identity_namespace: IdentityNamespace,
	workspace_limit: ByteCount,
}

impl<'a> MaterializationRequest<'a> {
	#[must_use]
	pub const fn new(
		descriptor: OperationDescriptor,
		inputs: &'a [NamedTensor<'a>],
		outputs: &'a [NamedTensor<'a>],
		iteration_shape_input: &'a str,
		parameters: &'a PreparedParameters,
		identity_namespace: IdentityNamespace,
		workspace_limit: ByteCount,
	) -> Self {
		Self {
			descriptor,
			inputs,
			outputs,
			iteration_shape_input,
			parameters,
			identity_namespace,
			workspace_limit,
		}
	}
}

/// One statically resolved repeat invocation surrounding a primitive step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResolvedIteration {
	role: &'static str,
	index: u64,
	count: u64,
}

impl ResolvedIteration {
	#[must_use]
	pub const fn role(self) -> &'static str {
		self.role
	}

	#[must_use]
	pub const fn index(self) -> u64 {
		self.index
	}

	#[must_use]
	pub const fn count(self) -> u64 {
		self.count
	}
}

/// One repeat bound and its exact preparation-time value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedBound {
	role: &'static str,
	bound: IterationBound,
	value: u64,
	outer_iterations: Vec<ResolvedIteration>,
}

impl ResolvedBound {
	#[must_use]
	pub const fn role(&self) -> &'static str {
		self.role
	}

	#[must_use]
	pub const fn bound(&self) -> IterationBound {
		self.bound
	}

	#[must_use]
	pub const fn value(&self) -> u64 {
		self.value
	}

	#[must_use]
	pub fn outer_iterations(&self) -> &[ResolvedIteration] {
		&self.outer_iterations
	}
}

/// One primitive after all surrounding repeats have been unrolled.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedStep {
	ordinal: usize,
	family: PrimitiveFamily,
	role: &'static str,
	iterations: Vec<ResolvedIteration>,
	dependencies: Vec<usize>,
}

impl ResolvedStep {
	#[must_use]
	pub const fn ordinal(&self) -> usize {
		self.ordinal
	}

	#[must_use]
	pub const fn family(&self) -> PrimitiveFamily {
		self.family
	}

	#[must_use]
	pub const fn role(&self) -> &'static str {
		self.role
	}

	#[must_use]
	pub fn iterations(&self) -> &[ResolvedIteration] {
		&self.iterations
	}

	#[must_use]
	pub fn dependencies(&self) -> &[usize] {
		&self.dependencies
	}
}

/// Finite dependency expansion of a descriptive composition recipe.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedComposition {
	operation: OperationId,
	bounds: Vec<ResolvedBound>,
	steps: Vec<ResolvedStep>,
}

impl ResolvedComposition {
	#[must_use]
	pub const fn operation(&self) -> OperationId {
		self.operation
	}

	#[must_use]
	pub fn bounds(&self) -> &[ResolvedBound] {
		&self.bounds
	}

	#[must_use]
	pub fn steps(&self) -> &[ResolvedStep] {
		&self.steps
	}
}

/// One intermediate tensor reserved before the run loop.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceObject {
	value: ValueId,
	dtype: DType,
	shape: Shape,
	bytes: ByteCount,
}

impl WorkspaceObject {
	#[must_use]
	pub const fn value(&self) -> ValueId {
		self.value
	}

	#[must_use]
	pub const fn dtype(&self) -> DType {
		self.dtype
	}

	#[must_use]
	pub const fn shape(&self) -> &Shape {
		&self.shape
	}

	#[must_use]
	pub const fn bytes(&self) -> ByteCount {
		self.bytes
	}
}

/// Checked, exact scratch reservation for the emitted graph.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceAllocation {
	objects: Vec<WorkspaceObject>,
	bytes: ByteCount,
}

impl WorkspaceAllocation {
	#[must_use]
	pub fn objects(&self) -> &[WorkspaceObject] {
		&self.objects
	}

	#[must_use]
	pub const fn bytes(&self) -> ByteCount {
		self.bytes
	}
}

/// Mapping from one resolved composition step to its concrete kernel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StageEmission {
	step: usize,
	kernels: Vec<KernelTemplateId>,
}

impl StageEmission {
	#[must_use]
	pub const fn step(&self) -> usize {
		self.step
	}

	#[must_use]
	pub fn kernels(&self) -> &[KernelTemplateId] {
		&self.kernels
	}
}

/// Fully concrete, validated output of composition preparation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaterializedComposition {
	operation: OperationId,
	graph: CalculationGraph,
	resolved: ResolvedComposition,
	stages: Vec<StageEmission>,
	workspace: WorkspaceAllocation,
	identity_namespace: IdentityNamespace,
}

impl MaterializedComposition {
	#[must_use]
	pub const fn operation(&self) -> OperationId {
		self.operation
	}

	#[must_use]
	pub const fn graph(&self) -> &CalculationGraph {
		&self.graph
	}

	#[must_use]
	pub const fn resolved(&self) -> &ResolvedComposition {
		&self.resolved
	}

	#[must_use]
	pub fn stages(&self) -> &[StageEmission] {
		&self.stages
	}

	#[must_use]
	pub const fn workspace(&self) -> &WorkspaceAllocation {
		&self.workspace
	}

	#[must_use]
	pub const fn identity_namespace(&self) -> IdentityNamespace {
		self.identity_namespace
	}
}

/// Reject overlapping caller reservations before independently materialized
/// fragments are assembled into one calculation graph.
pub fn validate_identity_namespaces(namespaces: &[IdentityNamespace]) -> OperationResult<()> {
	let ranges = namespaces
		.iter()
		.copied()
		.map(identity_ranges)
		.collect::<OperationResult<Vec<_>>>()?;
	for left_index in 0..ranges.len() {
		for right_index in left_index + 1..ranges.len() {
			let left = ranges[left_index];
			let right = ranges[right_index];
			if ranges_overlap(left.0, left.1, right.0, right.1) {
				return Err(OperationError::new(
					OperationErrorKind::IdentityNamespaceOverlap,
					format!(
						"intermediate value ranges for namespaces {left_index} and {right_index} overlap"
					),
				));
			}
			if ranges_overlap(left.2, left.3, right.2, right.3) {
				return Err(OperationError::new(
					OperationErrorKind::IdentityNamespaceOverlap,
					format!("kernel ranges for namespaces {left_index} and {right_index} overlap"),
				));
			}
		}
	}
	Ok(())
}

/// Concrete information still absent for a structured surface operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MissingConcreteComponent {
	TensorAbi,
	ScalarFormula,
	PrimitiveParameters,
	WorkspacePolicy,
}

/// Exact source-qualified manifest row for a composition that still fails
/// closed at materialization.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RemainingComposition {
	operation: OperationId,
	symbol: &'static str,
	source: &'static str,
	recipe: &'static str,
	missing: &'static [MissingConcreteComponent],
}

impl RemainingComposition {
	#[must_use]
	pub const fn operation(self) -> OperationId {
		self.operation
	}

	#[must_use]
	pub const fn symbol(self) -> &'static str {
		self.symbol
	}

	#[must_use]
	pub const fn source(self) -> &'static str {
		self.source
	}

	#[must_use]
	pub const fn recipe(self) -> &'static str {
		self.recipe
	}

	#[must_use]
	pub const fn missing(self) -> &'static [MissingConcreteComponent] {
		self.missing
	}
}

/// Resolve all shape and prepared-parameter bounds and unroll the composition
/// into a finite, deterministic dependency chain.
pub fn expand_composition(
	descriptor: OperationDescriptor,
	iteration_shape: &Shape,
	parameters: &PreparedParameters,
) -> OperationResult<ResolvedComposition> {
	let recipe = match descriptor.lowering {
		LoweringAvailability::Composition(recipe) => recipe,
		lowering => {
			return Err(operation_error(
				descriptor.id,
				OperationErrorKind::WrongLoweringKind,
				format!("operation lowering {lowering:?} is not a structured composition"),
			));
		}
	};
	recipe.validate()
		.map_err(|error| error.for_operation(descriptor.id))?;
	let mut expansion = Expansion::new(descriptor.id, iteration_shape, parameters);
	expansion.expand(recipe.steps(), &[])?;
	Ok(ResolvedComposition {
		operation: descriptor.id,
		bounds: expansion.bounds,
		steps: expansion.steps,
	})
}

/// Materialize one supported source operation. Descriptive recipes without an
/// operation-specific tensor ABI and scalar SSA program fail closed.
pub fn materialize_composition(request: MaterializationRequest<'_>) -> OperationResult<MaterializedComposition> {
	validate_request(&request)?;
	if !has_concrete_materializer(request.descriptor) {
		return Err(operation_error(
			request.descriptor.id,
			OperationErrorKind::MissingConcreteFormula,
			format!(
				"{} ({}) remains in remaining_composition_manifest: tensor ABI, scalar SSA, primitive parameters, and workspace policy are not concrete",
				request.descriptor.symbol, request.descriptor.source
			),
		));
	}
	let iteration_shape = request
		.inputs
		.iter()
		.find_map(|input| (input.name == request.iteration_shape_input).then_some(&input.tensor.shape))
		.ok_or_else(|| {
			operation_error(
				request.descriptor.id,
				OperationErrorKind::InvalidMaterializationRequest,
				format!(
					"iteration shape input {:?} is not an input declaration",
					request.iteration_shape_input
				),
			)
		})?;
	let resolved = expand_composition(request.descriptor, iteration_shape, request.parameters)?;
	let mut emitter = Emitter::new(&request, &resolved)?;
	dispatch_concrete(&request, &mut emitter)?;
	emitter.finish(&resolved)
}

enum FamilyDispatch {
	NotOwned,
	Owned(OperationResult<()>),
}

fn dispatch_concrete(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	macro_rules! dispatch_family {
		($family:ident) => {
			match $family::dispatch(request, emitter) {
				FamilyDispatch::NotOwned => {}
				FamilyDispatch::Owned(result) => return result,
			}
		};
	}
	dispatch_family!(optimizer_normalization);
	dispatch_family!(solver_fft);
	dispatch_family!(attention_sequence_embedding);
	dispatch_family!(convolution_pooling);
	dispatch_family!(loss_metrics);
	dispatch_family!(indexing_sort_encoding);
	dispatch_family!(graph_cluster_rl);
	dispatch_family!(tree_boosting);
	dispatch_family!(inference_quantization_diffusion);
	dispatch_family!(creation_shape_misc);
	Err(operation_error(
		request.descriptor.id,
		OperationErrorKind::GraphMaterializationFailed,
		format!(
			"concrete dispatch is missing for source-qualified operation {}",
			request.descriptor.symbol
		),
	))
}

/// Return every source-qualified structured operation that cannot yet be
/// materialized without inventing semantics.
#[must_use]
pub fn remaining_composition_manifest() -> Vec<RemainingComposition> {
	operation_registry()
		.iter()
		.filter_map(|descriptor| {
			let recipe = match descriptor.lowering {
				LoweringAvailability::Composition(recipe) => recipe,
				LoweringAvailability::Scalar(_)
				| LoweringAvailability::Primitive(_)
				| LoweringAvailability::Workspace(_)
				| LoweringAvailability::NonCalculation(_)
				| LoweringAvailability::Unsupported(_) => return None,
			};
			(!has_concrete_materializer(descriptor)).then_some(RemainingComposition {
				operation: descriptor.id,
				symbol: descriptor.symbol,
				source: descriptor.source,
				recipe: recipe.name(),
				missing: MISSING_COMPONENTS,
			})
		})
		.collect()
}

struct Expansion<'a> {
	operation: OperationId,
	shape: &'a Shape,
	parameters: &'a PreparedParameters,
	bounds: Vec<ResolvedBound>,
	steps: Vec<ResolvedStep>,
}

impl<'a> Expansion<'a> {
	const fn new(operation: OperationId, shape: &'a Shape, parameters: &'a PreparedParameters) -> Self {
		Self {
			operation,
			shape,
			parameters,
			bounds: Vec::new(),
			steps: Vec::new(),
		}
	}

	fn expand(&mut self, steps: &[CompositionStep], iterations: &[ResolvedIteration]) -> OperationResult<()> {
		for step in steps {
			match step {
				CompositionStep::Primitive { family, role } => {
					self.push_primitive(*family, role, iterations)?
				}
				CompositionStep::Repeat { bound, role, body } => {
					let count = self.resolve_bound(*bound)?;
					self.bounds.push(ResolvedBound {
						role,
						bound: *bound,
						value: count,
						outer_iterations: iterations.to_vec(),
					});
					for index in 0..count {
						let mut nested = iterations.to_vec();
						nested.push(ResolvedIteration { role, index, count });
						self.expand(body, &nested)?;
					}
				}
			}
		}
		Ok(())
	}

	fn push_primitive(
		&mut self,
		family: PrimitiveFamily,
		role: &'static str,
		iterations: &[ResolvedIteration],
	) -> OperationResult<()> {
		if self.steps.len() == MAX_EXPANDED_STEPS {
			return Err(operation_error(
				self.operation,
				OperationErrorKind::CompositionExpansionOverflow,
				format!("composition exceeds the fixed {MAX_EXPANDED_STEPS}-step preparation limit"),
			));
		}
		let ordinal = self.steps.len();
		let dependencies = ordinal.checked_sub(1).into_iter().collect();
		self.steps.push(ResolvedStep {
			ordinal,
			family,
			role,
			iterations: iterations.to_vec(),
			dependencies,
		});
		Ok(())
	}

	fn resolve_bound(&self, bound: IterationBound) -> OperationResult<u64> {
		match bound {
			IterationBound::Fixed(value) => Ok(u64::from(value)),
			IterationBound::ShapeExtent { axis } => self.extent(axis),
			IterationBound::MinimumShapeExtent => self.shape.extents().iter().copied().min().ok_or_else(|| {
				operation_error(
					self.operation,
					OperationErrorKind::IterationBoundUnresolved,
					"minimum shape extent requires a nonempty shape",
				)
			}),
			IterationBound::CeilingLog2ShapeExtent { axis } => self.extent(axis).map(ceiling_log_two),
			IterationBound::PreparedParameter { name } => prepared_u64(self.operation, self.parameters, name),
		}
	}

	fn extent(&self, axis: usize) -> OperationResult<u64> {
		self.shape.extents().get(axis).copied().ok_or_else(|| {
			operation_error(
				self.operation,
				OperationErrorKind::IterationBoundUnresolved,
				format!(
					"iteration axis {axis} is outside shape rank {}",
					self.shape.rank()
				),
			)
		})
	}
}

struct Emitter<'a> {
	operation: OperationId,
	resolved: &'a ResolvedComposition,
	cursor: usize,
	graph: GraphBuilder,
	stages: Vec<StageEmission>,
}

struct KernelEmission {
	inputs: Vec<ValueId>,
	outputs: Vec<ValueId>,
	kind: PrimitiveKind,
}

impl<'a> Emitter<'a> {
	fn new(request: &MaterializationRequest<'_>, resolved: &'a ResolvedComposition) -> OperationResult<Self> {
		Ok(Self {
			operation: request.descriptor.id,
			resolved,
			cursor: 0,
			graph: GraphBuilder::new(request)?,
			stages: Vec::with_capacity(resolved.steps.len()),
		})
	}

	fn intermediate(&mut self, dtype: DType, shape: Shape) -> OperationResult<ValueId> {
		self.graph.intermediate(dtype, shape)
	}

	fn emit(&mut self, inputs: Vec<ValueId>, outputs: Vec<ValueId>, kind: PrimitiveKind) -> OperationResult<()> {
		self.emit_stage([KernelEmission {
			inputs,
			outputs,
			kind,
		}])
	}

	fn emit_stage(&mut self, kernels: impl IntoIterator<Item = KernelEmission>) -> OperationResult<()> {
		let step = self.resolved.steps.get(self.cursor).ok_or_else(|| {
			operation_error(
				self.operation,
				OperationErrorKind::GraphMaterializationFailed,
				"concrete materializer emitted more kernels than the resolved recipe",
			)
		})?;
		let kernels = kernels.into_iter().collect::<Vec<_>>();
		let primary = kernels.last().ok_or_else(|| {
			operation_error(
				self.operation,
				OperationErrorKind::GraphMaterializationFailed,
				"a resolved stage cannot emit an empty kernel sequence",
			)
		})?;
		let family = primitive_family(&primary.kind);
		if family != step.family {
			return Err(operation_error(
				self.operation,
				OperationErrorKind::GraphMaterializationFailed,
				format!(
					"resolved step {} requires {:?}, concrete kernel is {:?}",
					step.ordinal, step.family, family
				),
			));
		}
		let mut emitted = Vec::with_capacity(kernels.len());
		for kernel in kernels {
			emitted.push(self
				.graph
				.emit(kernel.inputs, kernel.outputs, kernel.kind)?);
		}
		self.stages.push(StageEmission {
			step: step.ordinal,
			kernels: emitted,
		});
		self.cursor += 1;
		Ok(())
	}

	fn finish(self, resolved: &ResolvedComposition) -> OperationResult<MaterializedComposition> {
		if self.cursor != self.resolved.steps.len() {
			return Err(operation_error(
				self.operation,
				OperationErrorKind::GraphMaterializationFailed,
				format!(
					"concrete materializer emitted {} of {} resolved steps",
					self.cursor,
					self.resolved.steps.len()
				),
			));
		}
		let identity_namespace = self.graph.identity_namespace;
		let (graph, workspace) = self.graph.finish()?;
		Ok(MaterializedComposition {
			operation: self.operation,
			graph,
			resolved: resolved.clone(),
			stages: self.stages,
			workspace,
			identity_namespace,
		})
	}
}

struct GraphBuilder {
	operation: OperationId,
	tensors: Vec<Tensor>,
	nodes: Vec<CalculationNode>,
	next_value: u64,
	value_end: u64,
	next_kernel: u64,
	kernel_end: u64,
	workspace: Vec<WorkspaceObject>,
	workspace_bytes: u64,
	workspace_limit: ByteCount,
	identity_namespace: IdentityNamespace,
}

impl GraphBuilder {
	fn new(request: &MaterializationRequest<'_>) -> OperationResult<Self> {
		let tensors = request
			.inputs
			.iter()
			.chain(request.outputs)
			.map(|declaration| declaration.tensor.clone())
			.collect::<Vec<_>>();
		let (first_value, value_end, first_kernel, kernel_end) = identity_ranges(request.identity_namespace)
			.map_err(|error| error.for_operation(request.descriptor.id))?;
		for tensor in &tensors {
			if tensor.id.get() >= first_value && tensor.id.get() < value_end {
				return Err(operation_error(
					request.descriptor.id,
					OperationErrorKind::IdentityNamespaceOverlap,
					format!(
						"declared tensor {} falls inside reserved intermediate value range [{first_value}, {value_end})",
						tensor.id
					),
				));
			}
		}
		Ok(Self {
			operation: request.descriptor.id,
			tensors,
			nodes: Vec::new(),
			next_value: first_value,
			value_end,
			next_kernel: first_kernel,
			kernel_end,
			workspace: Vec::new(),
			workspace_bytes: 0,
			workspace_limit: request.workspace_limit,
			identity_namespace: request.identity_namespace,
		})
	}

	fn intermediate(&mut self, dtype: DType, shape: Shape) -> OperationResult<ValueId> {
		if self.next_value == self.value_end {
			return Err(operation_error(
				self.operation,
				OperationErrorKind::IdentityNamespaceExhausted,
				"reserved intermediate value identity range is exhausted",
			));
		}
		let value = ValueId::new(self.next_value);
		self.next_value += 1;
		let tensor = Tensor::contiguous(value, dtype, shape.clone(), false, false)
			.map_err(|error| language_error(self.operation, error.to_string()))?;
		let bytes = tensor.storage_bytes;
		let total = self
			.workspace_bytes
			.checked_add(bytes.get())
			.ok_or_else(|| {
				operation_error(
					self.operation,
					OperationErrorKind::WorkspaceArithmeticOverflow,
					"composition workspace byte total overflowed u64",
				)
			})?;
		if total > self.workspace_limit.get() {
			return Err(operation_error(
				self.operation,
				OperationErrorKind::WorkspaceLimitExceeded,
				format!(
					"composition needs {total} workspace bytes, limit is {}",
					self.workspace_limit.get()
				),
			));
		}
		self.workspace_bytes = total;
		self.workspace.push(WorkspaceObject {
			value,
			dtype,
			shape,
			bytes,
		});
		self.tensors.push(tensor);
		Ok(value)
	}

	fn emit(
		&mut self,
		inputs: Vec<ValueId>,
		outputs: Vec<ValueId>,
		kind: PrimitiveKind,
	) -> OperationResult<KernelTemplateId> {
		if self.next_kernel == self.kernel_end {
			return Err(operation_error(
				self.operation,
				OperationErrorKind::IdentityNamespaceExhausted,
				"reserved kernel identity range is exhausted",
			));
		}
		let id = KernelTemplateId::new(self.next_kernel);
		self.next_kernel += 1;
		let alias_rules = forbidden_aliases(inputs.len(), outputs.len());
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

	fn finish(self) -> OperationResult<(CalculationGraph, WorkspaceAllocation)> {
		let graph = CalculationGraph {
			tensors: self.tensors,
			nodes: self.nodes,
		};
		graph.validate()
			.map_err(|error| language_error(self.operation, error.to_string()))?;
		Ok((
			graph,
			WorkspaceAllocation {
				objects: self.workspace,
				bytes: ByteCount::new(self.workspace_bytes),
			},
		))
	}
}

fn emit_radix_two_fft(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	let real = input(request, "real")?;
	let imaginary = input(request, "imaginary")?;
	let transformed_real = output(request, "transformed_real")?;
	let transformed_imaginary = output(request, "transformed_imaginary")?;
	for (name, tensor) in [
		("real", real),
		("imaginary", imaginary),
		("transformed_real", transformed_real),
		("transformed_imaginary", transformed_imaginary),
	] {
		require_dtype(request, tensor, DType::F32, name)?;
	}
	if real.shape.rank() != 1 {
		return Err(operation_error(
			request.descriptor.id,
			OperationErrorKind::UnsupportedConcreteShape,
			"FFT real and imaginary payloads must be rank-one",
		));
	}
	require_same_tensor_contract(request, real, imaginary, "imaginary")?;
	require_same_tensor_contract(request, real, transformed_real, "transformed_real")?;
	require_same_tensor_contract(
		request,
		real,
		transformed_imaginary,
		"transformed_imaginary",
	)?;
	let elements = real.shape.elements();
	if elements < 2 || !elements.is_power_of_two() {
		return Err(operation_error(
			request.descriptor.id,
			OperationErrorKind::UnsupportedConcreteShape,
			format!("FFT length {elements} must be a power of two of at least two"),
		));
	}
	if elements > MAX_I32_INDEX {
		return Err(operation_error(
			request.descriptor.id,
			OperationErrorKind::UnsupportedConcreteShape,
			"FFT length must fit checked int32 indexes",
		));
	}
	require_true(request, "fft_stage_tables_verified")?;
	require_true(request, "fft_destination_indices_unique")?;
	let stage_count = usize::try_from(ceiling_log_two(elements)).map_err(|error| {
		request_error(
			request,
			format!("FFT stage count does not fit usize: {error}"),
		)
	})?;
	let vector_shape = real.shape.clone();
	let mut current_real = real.id;
	let mut current_imaginary = imaginary.id;
	for stage in 0..stage_count {
		let left_indices_name = format!("stage_{stage}_left_indices");
		let right_indices_name = format!("stage_{stage}_right_indices");
		let left_indices = input(request, &left_indices_name)?;
		let right_indices = input(request, &right_indices_name)?;
		for (name, tensor) in [
			(left_indices_name.as_str(), left_indices),
			(right_indices_name.as_str(), right_indices),
		] {
			require_dtype(request, tensor, DType::I32, name)?;
			require_shape(request, tensor, &[elements], name)?;
		}

		let left_real = emitter.intermediate(DType::F32, vector_shape.clone())?;
		let left_imaginary = emitter.intermediate(DType::F32, vector_shape.clone())?;
		let right_real = emitter.intermediate(DType::F32, vector_shape.clone())?;
		let right_imaginary = emitter.intermediate(DType::F32, vector_shape.clone())?;
		let gather = |value, indices, result| KernelEmission {
			inputs: vec![value, indices],
			outputs: vec![result],
			kind: PrimitiveKind::Gather(Gather {
				axis: 0,
				bounds: IndexBounds::Reject,
			}),
		};
		emitter.emit_stage([
			gather(current_real, left_indices.id, left_real),
			gather(current_imaginary, left_indices.id, left_imaginary),
			gather(current_real, right_indices.id, right_real),
			gather(current_imaginary, right_indices.id, right_imaginary),
		])?;

		let coefficient_names = [
			format!("stage_{stage}_real_from_left_real"),
			format!("stage_{stage}_real_from_left_imaginary"),
			format!("stage_{stage}_real_from_right_real"),
			format!("stage_{stage}_real_from_right_imaginary"),
			format!("stage_{stage}_imaginary_from_left_real"),
			format!("stage_{stage}_imaginary_from_left_imaginary"),
			format!("stage_{stage}_imaginary_from_right_real"),
			format!("stage_{stage}_imaginary_from_right_imaginary"),
		];
		let coefficients = coefficient_names
			.iter()
			.map(|name| input(request, name))
			.collect::<OperationResult<Vec<_>>>()?;
		for (name, tensor) in coefficient_names.iter().zip(&coefficients) {
			require_dtype(request, tensor, DType::F32, name)?;
			require_shape(request, tensor, &[elements], name)?;
		}
		let updated_real = emitter.intermediate(DType::F32, vector_shape.clone())?;
		let updated_imaginary = emitter.intermediate(DType::F32, vector_shape.clone())?;
		let mut scalar_inputs = vec![left_real, left_imaginary, right_real, right_imaginary];
		scalar_inputs.extend(coefficients.iter().map(|tensor| tensor.id));
		emitter.emit(
			scalar_inputs,
			vec![updated_real, updated_imaginary],
			PrimitiveKind::Elementwise(Elementwise {
				program: fft_linear_stage_program(request.descriptor.id)?,
			}),
		)?;

		let real_base_name = format!("stage_{stage}_real_base");
		let imaginary_base_name = format!("stage_{stage}_imaginary_base");
		let destinations_name = format!("stage_{stage}_destination_indices");
		let real_base = input(request, &real_base_name)?;
		let imaginary_base = input(request, &imaginary_base_name)?;
		let destinations = input(request, &destinations_name)?;
		for (name, tensor) in [
			(real_base_name.as_str(), real_base),
			(imaginary_base_name.as_str(), imaginary_base),
		] {
			require_same_tensor_contract(request, real, tensor, name)?;
		}
		require_dtype(request, destinations, DType::I32, &destinations_name)?;
		require_shape(request, destinations, &[elements], &destinations_name)?;
		let final_stage = stage + 1 == stage_count;
		let next_real = if final_stage {
			transformed_real.id
		} else {
			emitter.intermediate(DType::F32, vector_shape.clone())?
		};
		let next_imaginary = if final_stage {
			transformed_imaginary.id
		} else {
			emitter.intermediate(DType::F32, vector_shape.clone())?
		};
		let scatter = |base, updates, result| KernelEmission {
			inputs: vec![base, destinations.id, updates],
			outputs: vec![result],
			kind: PrimitiveKind::Scatter(Scatter {
				axis: 0,
				bounds: IndexBounds::Reject,
				conflict: ScatterConflict::UniqueIndices,
			}),
		};
		emitter.emit_stage([
			scatter(real_base.id, updated_real, next_real),
			scatter(imaginary_base.id, updated_imaginary, next_imaginary),
		])?;
		current_real = next_real;
		current_imaginary = next_imaginary;
	}
	Ok(())
}

fn emit_triangular_solve(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	let solution_base = input(request, "solution_base")?;
	let solution = output(request, "solution")?;
	require_dtype(request, solution_base, DType::F32, "solution_base")?;
	require_same_tensor_contract(request, solution_base, solution, "solution")?;
	if solution_base.shape.rank() != 1 || solution_base.shape.is_empty() {
		return Err(operation_error(
			request.descriptor.id,
			OperationErrorKind::UnsupportedConcreteShape,
			"triangular solve requires a nonempty rank-one solution shape",
		));
	}
	require_true(request, "solution_base_zero")?;
	require_true(request, "triangular_rows_verified")?;
	require_true(request, "destination_indices_unique")?;
	let tree_lanes = prepared_tree_lanes(request)?;
	let scalar_shape =
		Shape::new(vec![1]).map_err(|error| language_error(request.descriptor.id, error.to_string()))?;
	let axes = AxisSet::new(vec![0]).map_err(|error| language_error(request.descriptor.id, error.to_string()))?;
	let elements = solution_base.shape.elements();
	let mut current = solution_base.id;
	for row in 0..elements {
		let row_name = format!("row_{row}_strict_prefix");
		let rhs_name = format!("row_{row}_rhs");
		let diagonal_name = format!("row_{row}_diagonal");
		let destination_name = format!("row_{row}_destination");
		let coefficients = input(request, &row_name)?;
		let rhs = input(request, &rhs_name)?;
		let diagonal = input(request, &diagonal_name)?;
		let destination = input(request, &destination_name)?;
		require_dtype(request, coefficients, DType::F32, &row_name)?;
		require_shape(request, coefficients, &[elements], &row_name)?;
		for (name, tensor, dtype) in [
			(rhs_name.as_str(), rhs, DType::F32),
			(diagonal_name.as_str(), diagonal, DType::F32),
			(destination_name.as_str(), destination, DType::I32),
		] {
			require_dtype(request, tensor, dtype, name)?;
			require_shape(request, tensor, &[1], name)?;
		}

		let terms = emitter.intermediate(DType::F32, solution_base.shape.clone())?;
		let prefix = emitter.intermediate(DType::F32, scalar_shape.clone())?;
		emitter.emit_stage([
			KernelEmission {
				inputs: vec![coefficients.id, current],
				outputs: vec![terms],
				kind: PrimitiveKind::Elementwise(Elementwise {
					program: multiply_program(request.descriptor.id)?,
				}),
			},
			KernelEmission {
				inputs: vec![terms],
				outputs: vec![prefix],
				kind: PrimitiveKind::Reduce(Reduce {
					operator: ReduceOperator::Sum,
					axes: axes.clone(),
					keep_dimensions: false,
					result: ReduceResult::Value,
					tree_lanes,
				}),
			},
		])?;
		let solved = emitter.intermediate(DType::F32, scalar_shape.clone())?;
		emitter.emit(
			vec![rhs.id, prefix, diagonal.id],
			vec![solved],
			PrimitiveKind::Elementwise(Elementwise {
				program: triangular_solve_program(request.descriptor.id)?,
			}),
		)?;
		let next = if row + 1 == elements {
			solution.id
		} else {
			emitter.intermediate(DType::F32, solution_base.shape.clone())?
		};
		emitter.emit(
			vec![current, destination.id, solved],
			vec![next],
			PrimitiveKind::Scatter(Scatter {
				axis: 0,
				bounds: IndexBounds::Reject,
				conflict: ScatterConflict::UniqueIndices,
			}),
		)?;
		current = next;
	}
	Ok(())
}

fn emit_checked_gather_identity(
	request: &MaterializationRequest<'_>,
	emitter: &mut Emitter<'_>,
	values_name: &str,
	indices_name: &str,
	output_name: &str,
	axis_parameter: &str,
	required_fact: Option<&str>,
) -> OperationResult<()> {
	let values = input(request, values_name)?;
	let indices = input(request, indices_name)?;
	let result = output(request, output_name)?;
	require_dtype(request, values, DType::F32, values_name)?;
	require_dtype(request, indices, DType::I32, indices_name)?;
	require_dtype(request, result, DType::F32, output_name)?;
	if let Some(name) = required_fact {
		require_true(request, name)?;
	}
	let axis = prepared_axis(request, axis_parameter)?;
	let expected = values
		.shape
		.gather_result(axis, &indices.shape)
		.map_err(|error| language_error(request.descriptor.id, error.to_string()))?;
	if result.shape != expected {
		return Err(request_error(
			request,
			format!(
				"{output_name} shape {:?} does not match checked gather shape {:?}",
				result.shape.extents(),
				expected.extents()
			),
		));
	}
	let gathered = emitter.intermediate(DType::F32, expected)?;
	emitter.emit(
		vec![values.id, indices.id],
		vec![gathered],
		PrimitiveKind::Gather(Gather {
			axis,
			bounds: IndexBounds::Reject,
		}),
	)?;
	emitter.emit(
		vec![gathered],
		vec![result.id],
		PrimitiveKind::Elementwise(Elementwise {
			program: identity_program(request.descriptor.id, DType::F32)?,
		}),
	)
}

fn emit_batch_normalization_inference(
	request: &MaterializationRequest<'_>,
	emitter: &mut Emitter<'_>,
) -> OperationResult<()> {
	require_exact_abi(
		request,
		&[
			"values",
			"running_mean",
			"running_variance",
			"scale",
			"bias",
		],
		&["normalized"],
		&["epsilon"],
	)?;
	let values = input(request, "values")?;
	let mean = input(request, "running_mean")?;
	let variance = input(request, "running_variance")?;
	let scale = input(request, "scale")?;
	let bias = input(request, "bias")?;
	let normalized = output(request, "normalized")?;
	let inputs = [values, mean, variance, scale, bias];
	for (name, tensor) in [
		("values", values),
		("running_mean", mean),
		("running_variance", variance),
		("scale", scale),
		("bias", bias),
		("normalized", normalized),
	] {
		require_dtype(request, tensor, DType::F32, name)?;
	}
	let expected = Shape::broadcast_result(&inputs.map(|tensor| &tensor.shape))
		.map_err(|error| language_error(request.descriptor.id, error.to_string()))?;
	if normalized.shape != expected {
		return Err(request_error(
			request,
			format!(
				"normalized shape {:?} does not match broadcast shape {:?}",
				normalized.shape.extents(),
				expected.extents()
			),
		));
	}
	let epsilon = prepared_f32(request, "epsilon")?;
	if !(epsilon.is_finite() && epsilon > 0.0) {
		return Err(request_error(
			request,
			"epsilon must be a finite positive f32",
		));
	}
	emitter.emit(
		inputs.into_iter().map(|tensor| tensor.id).collect(),
		vec![normalized.id],
		PrimitiveKind::Elementwise(Elementwise {
			program: batch_normalization_program(request.descriptor.id, epsilon)?,
		}),
	)
}

fn emit_batch_normalization_forward(
	request: &MaterializationRequest<'_>,
	emitter: &mut Emitter<'_>,
) -> OperationResult<()> {
	require_exact_abi(
		request,
		&["values", "scale", "bias"],
		&[
			"normalized",
			"saved_mean",
			"saved_inverse_standard_deviation",
		],
		&["epsilon", "tree_lanes"],
	)?;
	let values = input(request, "values")?;
	let scale = input(request, "scale")?;
	let bias = input(request, "bias")?;
	let normalized = output(request, "normalized")?;
	let saved_mean = output(request, "saved_mean")?;
	let saved_inverse_standard_deviation = output(request, "saved_inverse_standard_deviation")?;
	let (rows, columns, row_divisor, _) = normalization_matrix(request, values, "values")?;
	require_same_tensor_contract(request, values, normalized, "normalized")?;
	for (name, tensor) in [
		("scale", scale),
		("bias", bias),
		("saved_mean", saved_mean),
		(
			"saved_inverse_standard_deviation",
			saved_inverse_standard_deviation,
		),
	] {
		require_dtype(request, tensor, DType::F32, name)?;
		require_shape(request, tensor, &[columns], name)?;
	}
	let epsilon = finite_positive_parameter(request, "epsilon")?;
	let tree_lanes = prepared_tree_lanes(request)?;
	let statistic_values = emitter.intermediate(DType::F32, values.shape.clone())?;
	let channel_shape =
		Shape::new(vec![columns]).map_err(|error| language_error(request.descriptor.id, error.to_string()))?;
	let sums = emitter.intermediate(DType::F32, channel_shape.clone())?;
	let centered_squares = emitter.intermediate(DType::F32, values.shape.clone())?;
	let variance_sums = emitter.intermediate(DType::F32, channel_shape)?;
	emitter.emit(
		vec![values.id],
		vec![statistic_values],
		PrimitiveKind::Elementwise(Elementwise {
			program: identity_program(request.descriptor.id, DType::F32)?,
		}),
	)?;
	let channel_axis =
		AxisSet::new(vec![0]).map_err(|error| language_error(request.descriptor.id, error.to_string()))?;
	emitter.emit(
		vec![statistic_values],
		vec![sums],
		sum_reduction(request, channel_axis.clone(), false, tree_lanes)?,
	)?;
	emitter.emit_stage([
		KernelEmission {
			inputs: vec![values.id, sums],
			outputs: vec![centered_squares],
			kind: PrimitiveKind::Elementwise(Elementwise {
				program: centered_square_program(request.descriptor.id, row_divisor)?,
			}),
		},
		KernelEmission {
			inputs: vec![centered_squares],
			outputs: vec![variance_sums],
			kind: sum_reduction(request, channel_axis, false, tree_lanes)?,
		},
	])?;
	emitter.emit_stage([
		KernelEmission {
			inputs: vec![sums, variance_sums],
			outputs: vec![saved_mean.id, saved_inverse_standard_deviation.id],
			kind: PrimitiveKind::Elementwise(Elementwise {
				program: normalization_statistics_program(request.descriptor.id, row_divisor, epsilon)?,
			}),
		},
		KernelEmission {
			inputs: vec![
				values.id,
				scale.id,
				bias.id,
				saved_mean.id,
				saved_inverse_standard_deviation.id,
			],
			outputs: vec![normalized.id],
			kind: PrimitiveKind::Elementwise(Elementwise {
				program: affine_normalization_program(request.descriptor.id)?,
			}),
		},
	])?;
	if rows == 0 {
		return Err(request_error(
			request,
			"batch normalization rows must be nonzero",
		));
	}
	Ok(())
}

fn emit_batch_normalization_backward(
	request: &MaterializationRequest<'_>,
	emitter: &mut Emitter<'_>,
) -> OperationResult<()> {
	require_exact_abi(
		request,
		&[
			"output_gradient",
			"values",
			"saved_mean",
			"saved_inverse_standard_deviation",
			"scale",
		],
		&["input_gradient", "scale_gradient", "bias_gradient"],
		&["tree_lanes"],
	)?;
	let output_gradient = input(request, "output_gradient")?;
	let values = input(request, "values")?;
	let saved_mean = input(request, "saved_mean")?;
	let saved_inverse_standard_deviation = input(request, "saved_inverse_standard_deviation")?;
	let scale = input(request, "scale")?;
	let input_gradient = output(request, "input_gradient")?;
	let scale_gradient = output(request, "scale_gradient")?;
	let bias_gradient = output(request, "bias_gradient")?;
	let (_, columns, row_divisor, _) = normalization_matrix(request, values, "values")?;
	require_same_tensor_contract(request, values, output_gradient, "output_gradient")?;
	require_same_tensor_contract(request, values, input_gradient, "input_gradient")?;
	for (name, tensor) in [
		("saved_mean", saved_mean),
		(
			"saved_inverse_standard_deviation",
			saved_inverse_standard_deviation,
		),
		("scale", scale),
		("scale_gradient", scale_gradient),
		("bias_gradient", bias_gradient),
	] {
		require_dtype(request, tensor, DType::F32, name)?;
		require_shape(request, tensor, &[columns], name)?;
	}
	let tree_lanes = prepared_tree_lanes(request)?;
	let output_gradient_terms = emitter.intermediate(DType::F32, values.shape.clone())?;
	let normalized_gradient_terms = emitter.intermediate(DType::F32, values.shape.clone())?;
	let channel_shape =
		Shape::new(vec![columns]).map_err(|error| language_error(request.descriptor.id, error.to_string()))?;
	let output_gradient_sums = emitter.intermediate(DType::F32, channel_shape.clone())?;
	let normalized_gradient_sums = emitter.intermediate(DType::F32, channel_shape)?;
	let scale_gradient_terms = emitter.intermediate(DType::F32, values.shape.clone())?;
	let bias_gradient_terms = emitter.intermediate(DType::F32, values.shape.clone())?;
	emitter.emit(
		vec![
			output_gradient.id,
			values.id,
			saved_mean.id,
			saved_inverse_standard_deviation.id,
		],
		vec![output_gradient_terms, normalized_gradient_terms],
		PrimitiveKind::Elementwise(Elementwise {
			program: batch_normalization_backward_terms_program(request.descriptor.id)?,
		}),
	)?;
	let row_axis = AxisSet::new(vec![0]).map_err(|error| language_error(request.descriptor.id, error.to_string()))?;
	emitter.emit(
		vec![output_gradient_terms],
		vec![output_gradient_sums],
		sum_reduction(request, row_axis.clone(), false, tree_lanes)?,
	)?;
	emitter.emit(
		vec![normalized_gradient_terms],
		vec![normalized_gradient_sums],
		sum_reduction(request, row_axis.clone(), false, tree_lanes)?,
	)?;
	emitter.emit(
		vec![
			output_gradient.id,
			values.id,
			saved_mean.id,
			saved_inverse_standard_deviation.id,
			scale.id,
			output_gradient_sums,
			normalized_gradient_sums,
		],
		vec![input_gradient.id, scale_gradient_terms, bias_gradient_terms],
		PrimitiveKind::Elementwise(Elementwise {
			program: batch_normalization_gradient_program(request.descriptor.id, row_divisor)?,
		}),
	)?;
	emitter.emit_stage([
		KernelEmission {
			inputs: vec![scale_gradient_terms],
			outputs: vec![scale_gradient.id],
			kind: sum_reduction(request, row_axis.clone(), false, tree_lanes)?,
		},
		KernelEmission {
			inputs: vec![bias_gradient_terms],
			outputs: vec![bias_gradient.id],
			kind: sum_reduction(request, row_axis, false, tree_lanes)?,
		},
	])
}

fn emit_batch_normalization_running_update(
	request: &MaterializationRequest<'_>,
	emitter: &mut Emitter<'_>,
) -> OperationResult<()> {
	require_exact_abi(
		request,
		&[
			"saved_mean",
			"saved_variance",
			"running_mean",
			"running_variance",
		],
		&["updated_running_mean", "updated_running_variance"],
		&["momentum"],
	)?;
	let saved_mean = input(request, "saved_mean")?;
	let saved_variance = input(request, "saved_variance")?;
	let running_mean = input(request, "running_mean")?;
	let running_variance = input(request, "running_variance")?;
	let updated_running_mean = output(request, "updated_running_mean")?;
	let updated_running_variance = output(request, "updated_running_variance")?;
	require_equal_f32_tensors(
		request,
		running_mean,
		[
			("saved_mean", saved_mean),
			("saved_variance", saved_variance),
			("running_variance", running_variance),
			("updated_running_mean", updated_running_mean),
			("updated_running_variance", updated_running_variance),
		],
	)?;
	if running_mean.shape.rank() != 1 {
		return Err(operation_error(
			request.descriptor.id,
			OperationErrorKind::UnsupportedConcreteShape,
			"running batch-normalization statistics must be rank-one",
		));
	}
	let momentum = unit_interval_parameter(request, "momentum", true)?;
	emitter.emit(
		vec![
			running_mean.id,
			running_variance.id,
			saved_mean.id,
			saved_variance.id,
		],
		vec![updated_running_mean.id, updated_running_variance.id],
		PrimitiveKind::Elementwise(Elementwise {
			program: running_statistics_update_program(request.descriptor.id, momentum)?,
		}),
	)
}

fn emit_layer_normalization(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	let optional_affine = request.descriptor.symbol == "gpu_layernorm_opt_into";
	let has_scale = match optional_affine {
		true => prepared_bool(request, "has_scale")?,
		false => true,
	};
	let has_bias = match optional_affine {
		true => prepared_bool(request, "has_bias")?,
		false => true,
	};
	let mut input_names = vec!["values"];
	input_names.extend(has_scale.then_some("scale"));
	input_names.extend(has_bias.then_some("bias"));
	let parameter_names: &[&str] = match optional_affine {
		true => &["epsilon", "tree_lanes", "has_scale", "has_bias"],
		false => &["epsilon", "tree_lanes"],
	};
	require_exact_abi(request, &input_names, &["normalized"], parameter_names)?;
	let values = input(request, "values")?;
	let normalized = output(request, "normalized")?;
	let (rows, columns, _, column_divisor) = normalization_matrix(request, values, "values")?;
	require_same_tensor_contract(request, values, normalized, "normalized")?;
	let scale = match has_scale {
		true => Some(input(request, "scale")?),
		false => None,
	};
	let bias = match has_bias {
		true => Some(input(request, "bias")?),
		false => None,
	};
	for (name, tensor) in [("scale", scale), ("bias", bias)] {
		if let Some(tensor) = tensor {
			require_dtype(request, tensor, DType::F32, name)?;
			require_shape(request, tensor, &[columns], name)?;
		}
	}
	let epsilon = finite_positive_parameter(request, "epsilon")?;
	let tree_lanes = prepared_tree_lanes(request)?;
	let values_image = emitter.intermediate(DType::F32, values.shape.clone())?;
	let row_shape =
		Shape::new(vec![rows, 1]).map_err(|error| language_error(request.descriptor.id, error.to_string()))?;
	let sums = emitter.intermediate(DType::F32, row_shape.clone())?;
	let centered_squares = emitter.intermediate(DType::F32, values.shape.clone())?;
	let variance_sums = emitter.intermediate(DType::F32, row_shape)?;
	emitter.emit(
		vec![values.id],
		vec![values_image],
		PrimitiveKind::Elementwise(Elementwise {
			program: identity_program(request.descriptor.id, DType::F32)?,
		}),
	)?;
	let feature_axis =
		AxisSet::new(vec![1]).map_err(|error| language_error(request.descriptor.id, error.to_string()))?;
	emitter.emit(
		vec![values_image],
		vec![sums],
		sum_reduction(request, feature_axis.clone(), true, tree_lanes)?,
	)?;
	emitter.emit_stage([
		KernelEmission {
			inputs: vec![values.id, sums],
			outputs: vec![centered_squares],
			kind: PrimitiveKind::Elementwise(Elementwise {
				program: centered_square_program(request.descriptor.id, column_divisor)?,
			}),
		},
		KernelEmission {
			inputs: vec![centered_squares],
			outputs: vec![variance_sums],
			kind: sum_reduction(request, feature_axis, true, tree_lanes)?,
		},
	])?;
	let mut inputs = vec![values.id, sums, variance_sums];
	inputs.extend(scale.map(|tensor| tensor.id));
	inputs.extend(bias.map(|tensor| tensor.id));
	emitter.emit(
		inputs,
		vec![normalized.id],
		PrimitiveKind::Elementwise(Elementwise {
			program: layer_normalization_program(
				request.descriptor.id,
				column_divisor,
				epsilon,
				has_scale,
				has_bias,
			)?,
		}),
	)
}

fn emit_layer_normalization_backward(
	request: &MaterializationRequest<'_>,
	emitter: &mut Emitter<'_>,
) -> OperationResult<()> {
	require_exact_abi(
		request,
		&["output_gradient", "values", "scale"],
		&["input_gradient", "scale_gradient", "bias_gradient"],
		&["epsilon", "tree_lanes"],
	)?;
	let output_gradient = input(request, "output_gradient")?;
	let values = input(request, "values")?;
	let scale = input(request, "scale")?;
	let input_gradient = output(request, "input_gradient")?;
	let scale_gradient = output(request, "scale_gradient")?;
	let bias_gradient = output(request, "bias_gradient")?;
	let (rows, columns, _, column_divisor) = normalization_matrix(request, values, "values")?;
	require_same_tensor_contract(request, values, output_gradient, "output_gradient")?;
	require_same_tensor_contract(request, values, input_gradient, "input_gradient")?;
	for (name, tensor) in [
		("scale", scale),
		("scale_gradient", scale_gradient),
		("bias_gradient", bias_gradient),
	] {
		require_dtype(request, tensor, DType::F32, name)?;
		require_shape(request, tensor, &[columns], name)?;
	}
	let epsilon = finite_positive_parameter(request, "epsilon")?;
	let tree_lanes = prepared_tree_lanes(request)?;
	let values_image = emitter.intermediate(DType::F32, values.shape.clone())?;
	let row_shape =
		Shape::new(vec![rows, 1]).map_err(|error| language_error(request.descriptor.id, error.to_string()))?;
	let sums = emitter.intermediate(DType::F32, row_shape.clone())?;
	let centered_squares = emitter.intermediate(DType::F32, values.shape.clone())?;
	let variance_sums = emitter.intermediate(DType::F32, row_shape.clone())?;
	emitter.emit(
		vec![values.id],
		vec![values_image],
		PrimitiveKind::Elementwise(Elementwise {
			program: identity_program(request.descriptor.id, DType::F32)?,
		}),
	)?;
	let feature_axis =
		AxisSet::new(vec![1]).map_err(|error| language_error(request.descriptor.id, error.to_string()))?;
	emitter.emit(
		vec![values_image],
		vec![sums],
		sum_reduction(request, feature_axis.clone(), true, tree_lanes)?,
	)?;
	emitter.emit_stage([
		KernelEmission {
			inputs: vec![values.id, sums],
			outputs: vec![centered_squares],
			kind: PrimitiveKind::Elementwise(Elementwise {
				program: centered_square_program(request.descriptor.id, column_divisor)?,
			}),
		},
		KernelEmission {
			inputs: vec![centered_squares],
			outputs: vec![variance_sums],
			kind: sum_reduction(request, feature_axis.clone(), true, tree_lanes)?,
		},
	])?;
	let first_gradient_terms = emitter.intermediate(DType::F32, values.shape.clone())?;
	let second_gradient_terms = emitter.intermediate(DType::F32, values.shape.clone())?;
	let scale_gradient_terms = emitter.intermediate(DType::F32, values.shape.clone())?;
	let bias_gradient_terms = emitter.intermediate(DType::F32, values.shape.clone())?;
	let first_gradient_sums = emitter.intermediate(DType::F32, row_shape.clone())?;
	let second_gradient_sums = emitter.intermediate(DType::F32, row_shape)?;
	emitter.emit_stage([
		KernelEmission {
			inputs: vec![output_gradient.id, values.id, scale.id, sums, variance_sums],
			outputs: vec![
				first_gradient_terms,
				second_gradient_terms,
				scale_gradient_terms,
				bias_gradient_terms,
			],
			kind: PrimitiveKind::Elementwise(Elementwise {
				program: layer_normalization_backward_terms_program(
					request.descriptor.id,
					column_divisor,
					epsilon,
				)?,
			}),
		},
		KernelEmission {
			inputs: vec![first_gradient_terms],
			outputs: vec![first_gradient_sums],
			kind: sum_reduction(request, feature_axis.clone(), true, tree_lanes)?,
		},
		KernelEmission {
			inputs: vec![second_gradient_terms],
			outputs: vec![second_gradient_sums],
			kind: sum_reduction(request, feature_axis, true, tree_lanes)?,
		},
		KernelEmission {
			inputs: vec![
				output_gradient.id,
				values.id,
				scale.id,
				sums,
				variance_sums,
				first_gradient_sums,
				second_gradient_sums,
			],
			outputs: vec![input_gradient.id],
			kind: PrimitiveKind::Elementwise(Elementwise {
				program: layer_normalization_input_gradient_program(
					request.descriptor.id,
					column_divisor,
					epsilon,
				)?,
			}),
		},
	])?;
	let row_axis = AxisSet::new(vec![0]).map_err(|error| language_error(request.descriptor.id, error.to_string()))?;
	emitter.emit_stage([
		KernelEmission {
			inputs: vec![scale_gradient_terms],
			outputs: vec![scale_gradient.id],
			kind: sum_reduction(request, row_axis.clone(), false, tree_lanes)?,
		},
		KernelEmission {
			inputs: vec![bias_gradient_terms],
			outputs: vec![bias_gradient.id],
			kind: sum_reduction(request, row_axis, false, tree_lanes)?,
		},
	])
}

fn emit_rms_normalization(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	let has_scale = request.descriptor.symbol != "gpu_rmsnorm_f64_nogamma";
	let input_names: &[&str] = match has_scale {
		true => &["values", "scale"],
		false => &["values"],
	};
	require_exact_abi(
		request,
		input_names,
		&["normalized"],
		&["epsilon", "tree_lanes"],
	)?;
	let values = input(request, "values")?;
	let normalized = output(request, "normalized")?;
	let (rows, columns, _, column_divisor) = normalization_matrix(request, values, "values")?;
	require_same_tensor_contract(request, values, normalized, "normalized")?;
	let scale = match has_scale {
		true => Some(input(request, "scale")?),
		false => None,
	};
	if let Some(scale) = scale {
		require_dtype(request, scale, DType::F32, "scale")?;
		require_shape(request, scale, &[columns], "scale")?;
	}
	let epsilon = finite_positive_parameter(request, "epsilon")?;
	let tree_lanes = prepared_tree_lanes(request)?;
	let squares = emitter.intermediate(DType::F32, values.shape.clone())?;
	let row_shape =
		Shape::new(vec![rows, 1]).map_err(|error| language_error(request.descriptor.id, error.to_string()))?;
	let square_sums = emitter.intermediate(DType::F32, row_shape.clone())?;
	let fixed_square_sums = emitter.intermediate(DType::F32, row_shape)?;
	emitter.emit(
		vec![values.id],
		vec![squares],
		PrimitiveKind::Elementwise(Elementwise {
			program: square_program(request.descriptor.id)?,
		}),
	)?;
	let feature_axis =
		AxisSet::new(vec![1]).map_err(|error| language_error(request.descriptor.id, error.to_string()))?;
	emitter.emit(
		vec![squares],
		vec![square_sums],
		sum_reduction(request, feature_axis.clone(), true, tree_lanes)?,
	)?;
	emitter.emit(
		vec![square_sums],
		vec![fixed_square_sums],
		sum_reduction(request, feature_axis, true, tree_lanes)?,
	)?;
	let mut inputs = vec![values.id, fixed_square_sums];
	inputs.extend(scale.map(|tensor| tensor.id));
	emitter.emit(
		inputs,
		vec![normalized.id],
		PrimitiveKind::Elementwise(Elementwise {
			program: rms_normalization_program(request.descriptor.id, column_divisor, epsilon, has_scale)?,
		}),
	)
}

fn emit_rms_normalization_backward(
	request: &MaterializationRequest<'_>,
	emitter: &mut Emitter<'_>,
) -> OperationResult<()> {
	require_exact_abi(
		request,
		&["output_gradient", "values", "scale"],
		&["input_gradient", "scale_gradient"],
		&["epsilon", "tree_lanes"],
	)?;
	let output_gradient = input(request, "output_gradient")?;
	let values = input(request, "values")?;
	let scale = input(request, "scale")?;
	let input_gradient = output(request, "input_gradient")?;
	let scale_gradient = output(request, "scale_gradient")?;
	let (rows, columns, _, column_divisor) = normalization_matrix(request, values, "values")?;
	require_same_tensor_contract(request, values, output_gradient, "output_gradient")?;
	require_same_tensor_contract(request, values, input_gradient, "input_gradient")?;
	for (name, tensor) in [("scale", scale), ("scale_gradient", scale_gradient)] {
		require_dtype(request, tensor, DType::F32, name)?;
		require_shape(request, tensor, &[columns], name)?;
	}
	let epsilon = finite_positive_parameter(request, "epsilon")?;
	let tree_lanes = prepared_tree_lanes(request)?;
	let squares = emitter.intermediate(DType::F32, values.shape.clone())?;
	let row_shape =
		Shape::new(vec![rows, 1]).map_err(|error| language_error(request.descriptor.id, error.to_string()))?;
	let square_sums = emitter.intermediate(DType::F32, row_shape.clone())?;
	emitter.emit(
		vec![values.id],
		vec![squares],
		PrimitiveKind::Elementwise(Elementwise {
			program: square_program(request.descriptor.id)?,
		}),
	)?;
	let feature_axis =
		AxisSet::new(vec![1]).map_err(|error| language_error(request.descriptor.id, error.to_string()))?;
	emitter.emit(
		vec![squares],
		vec![square_sums],
		sum_reduction(request, feature_axis.clone(), true, tree_lanes)?,
	)?;
	let dot_terms = emitter.intermediate(DType::F32, values.shape.clone())?;
	let dot_sums = emitter.intermediate(DType::F32, row_shape)?;
	emitter.emit_stage([
		KernelEmission {
			inputs: vec![output_gradient.id, values.id, square_sums],
			outputs: vec![dot_terms],
			kind: PrimitiveKind::Elementwise(Elementwise {
				program: rms_dot_term_program(request.descriptor.id, column_divisor, epsilon)?,
			}),
		},
		KernelEmission {
			inputs: vec![dot_terms],
			outputs: vec![dot_sums],
			kind: sum_reduction(request, feature_axis, true, tree_lanes)?,
		},
	])?;
	let scale_gradient_terms = emitter.intermediate(DType::F32, values.shape.clone())?;
	emitter.emit(
		vec![
			output_gradient.id,
			values.id,
			scale.id,
			square_sums,
			dot_sums,
		],
		vec![input_gradient.id, scale_gradient_terms],
		PrimitiveKind::Elementwise(Elementwise {
			program: rms_normalization_backward_program(request.descriptor.id, column_divisor, epsilon)?,
		}),
	)?;
	let row_axis = AxisSet::new(vec![0]).map_err(|error| language_error(request.descriptor.id, error.to_string()))?;
	emitter.emit(
		vec![scale_gradient_terms],
		vec![scale_gradient.id],
		sum_reduction(request, row_axis, false, tree_lanes)?,
	)
}

fn emit_momentum_update(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(
		request,
		&["parameters", "velocity", "gradient"],
		&["updated_velocity", "updated_parameters"],
		&["momentum", "learning_rate"],
	)?;
	let parameters = input(request, "parameters")?;
	let velocity = input(request, "velocity")?;
	let gradient = input(request, "gradient")?;
	let updated_velocity = output(request, "updated_velocity")?;
	let updated_parameters = output(request, "updated_parameters")?;
	for (name, tensor) in [
		("parameters", parameters),
		("velocity", velocity),
		("gradient", gradient),
		("updated_velocity", updated_velocity),
		("updated_parameters", updated_parameters),
	] {
		require_dtype(request, tensor, DType::F32, name)?;
		require_same_tensor_contract(request, parameters, tensor, name)?;
	}
	let momentum = prepared_f32(request, "momentum")?;
	let learning_rate = prepared_f32(request, "learning_rate")?;
	if !(momentum.is_finite() && learning_rate.is_finite() && learning_rate >= 0.0) {
		return Err(request_error(
			request,
			"momentum must be finite and learning_rate must be finite and nonnegative",
		));
	}
	emitter.emit(
		vec![velocity.id, gradient.id],
		vec![updated_velocity.id],
		PrimitiveKind::Elementwise(Elementwise {
			program: momentum_program(request.descriptor.id, momentum, learning_rate)?,
		}),
	)?;
	emitter.emit(
		vec![parameters.id, updated_velocity.id],
		vec![updated_parameters.id],
		PrimitiveKind::Elementwise(Elementwise {
			program: momentum_weight_program(request.descriptor.id)?,
		}),
	)
}

fn emit_adagrad_update(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(
		request,
		&["gradient", "weight", "accumulator"],
		&["updated_accumulator", "updated_weight"],
		&["learning_rate", "epsilon"],
	)?;
	let gradient = input(request, "gradient")?;
	let weight = input(request, "weight")?;
	let accumulator = input(request, "accumulator")?;
	let updated_accumulator = output(request, "updated_accumulator")?;
	let updated_weight = output(request, "updated_weight")?;
	require_equal_f32_tensors(
		request,
		weight,
		[
			("gradient", gradient),
			("accumulator", accumulator),
			("updated_accumulator", updated_accumulator),
			("updated_weight", updated_weight),
		],
	)?;
	let learning_rate = finite_nonnegative_parameter(request, "learning_rate")?;
	let epsilon = finite_positive_parameter(request, "epsilon")?;
	emitter.emit(
		vec![accumulator.id, gradient.id],
		vec![updated_accumulator.id],
		PrimitiveKind::Elementwise(Elementwise {
			program: adagrad_state_program(request.descriptor.id)?,
		}),
	)?;
	emitter.emit(
		vec![weight.id, gradient.id, updated_accumulator.id],
		vec![updated_weight.id],
		PrimitiveKind::Elementwise(Elementwise {
			program: adaptive_weight_program(request.descriptor.id, learning_rate, epsilon)?,
		}),
	)
}

fn emit_rmsprop_update(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(
		request,
		&["gradient", "weight", "cache"],
		&["updated_cache", "updated_weight"],
		&["learning_rate", "decay", "epsilon"],
	)?;
	let gradient = input(request, "gradient")?;
	let weight = input(request, "weight")?;
	let cache = input(request, "cache")?;
	let updated_cache = output(request, "updated_cache")?;
	let updated_weight = output(request, "updated_weight")?;
	require_equal_f32_tensors(
		request,
		weight,
		[
			("gradient", gradient),
			("cache", cache),
			("updated_cache", updated_cache),
			("updated_weight", updated_weight),
		],
	)?;
	let learning_rate = finite_nonnegative_parameter(request, "learning_rate")?;
	let decay = unit_interval_parameter(request, "decay", true)?;
	let epsilon = finite_positive_parameter(request, "epsilon")?;
	emitter.emit(
		vec![cache.id, gradient.id],
		vec![updated_cache.id],
		PrimitiveKind::Elementwise(Elementwise {
			program: rmsprop_state_program(request.descriptor.id, decay)?,
		}),
	)?;
	emitter.emit(
		vec![weight.id, gradient.id, updated_cache.id],
		vec![updated_weight.id],
		PrimitiveKind::Elementwise(Elementwise {
			program: adaptive_weight_program(request.descriptor.id, learning_rate, epsilon)?,
		}),
	)
}

fn emit_adam_update(
	request: &MaterializationRequest<'_>,
	emitter: &mut Emitter<'_>,
	decoupled_weight_decay: bool,
) -> OperationResult<()> {
	let parameter_names: &[&str] = match decoupled_weight_decay {
		true => &[
			"learning_rate",
			"beta_one",
			"beta_two",
			"epsilon",
			"weight_decay",
			"step",
		],
		false => &["learning_rate", "beta_one", "beta_two", "epsilon", "step"],
	};
	require_exact_abi(
		request,
		&["gradient", "weight", "first_moment", "second_moment"],
		&[
			"updated_first_moment",
			"updated_second_moment",
			"updated_weight",
		],
		parameter_names,
	)?;
	let gradient = input(request, "gradient")?;
	let weight = input(request, "weight")?;
	let first_moment = input(request, "first_moment")?;
	let second_moment = input(request, "second_moment")?;
	let updated_first_moment = output(request, "updated_first_moment")?;
	let updated_second_moment = output(request, "updated_second_moment")?;
	let updated_weight = output(request, "updated_weight")?;
	require_equal_f32_tensors(
		request,
		weight,
		[
			("gradient", gradient),
			("first_moment", first_moment),
			("second_moment", second_moment),
			("updated_first_moment", updated_first_moment),
			("updated_second_moment", updated_second_moment),
			("updated_weight", updated_weight),
		],
	)?;
	let moments = adam_moment_parameters(request)?;
	let learning_rate = finite_nonnegative_parameter(request, "learning_rate")?;
	let weight_decay = match decoupled_weight_decay {
		true => finite_nonnegative_parameter(request, "weight_decay")?,
		false => 0.0,
	};
	emitter.emit(
		vec![first_moment.id, second_moment.id, gradient.id],
		vec![updated_first_moment.id, updated_second_moment.id],
		PrimitiveKind::Elementwise(Elementwise {
			program: adam_state_program(request.descriptor.id, moments.beta_one, moments.beta_two)?,
		}),
	)?;
	emitter.emit(
		vec![weight.id, updated_first_moment.id, updated_second_moment.id],
		vec![updated_weight.id],
		PrimitiveKind::Elementwise(Elementwise {
			program: adam_weight_program(
				request.descriptor.id,
				moments,
				learning_rate,
				weight_decay,
				decoupled_weight_decay,
			)?,
		}),
	)
}

fn emit_nadam_update(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(
		request,
		&["gradient", "weight", "first_moment", "second_moment"],
		&[
			"updated_first_moment",
			"updated_second_moment",
			"updated_weight",
		],
		&["learning_rate", "beta_one", "beta_two", "epsilon", "step"],
	)?;
	let gradient = input(request, "gradient")?;
	let weight = input(request, "weight")?;
	let first_moment = input(request, "first_moment")?;
	let second_moment = input(request, "second_moment")?;
	let updated_first_moment = output(request, "updated_first_moment")?;
	let updated_second_moment = output(request, "updated_second_moment")?;
	let updated_weight = output(request, "updated_weight")?;
	require_equal_f32_tensors(
		request,
		weight,
		[
			("gradient", gradient),
			("first_moment", first_moment),
			("second_moment", second_moment),
			("updated_first_moment", updated_first_moment),
			("updated_second_moment", updated_second_moment),
			("updated_weight", updated_weight),
		],
	)?;
	let moments = adam_moment_parameters(request)?;
	let learning_rate = finite_nonnegative_parameter(request, "learning_rate")?;
	emitter.emit(
		vec![first_moment.id, second_moment.id, gradient.id],
		vec![updated_first_moment.id, updated_second_moment.id],
		PrimitiveKind::Elementwise(Elementwise {
			program: adam_state_program(request.descriptor.id, moments.beta_one, moments.beta_two)?,
		}),
	)?;
	emitter.emit(
		vec![
			weight.id,
			gradient.id,
			updated_first_moment.id,
			updated_second_moment.id,
		],
		vec![updated_weight.id],
		PrimitiveKind::Elementwise(Elementwise {
			program: nadam_weight_program(request.descriptor.id, moments, learning_rate)?,
		}),
	)
}

fn emit_lion_update(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(
		request,
		&["gradient", "weight", "moment"],
		&["updated_moment", "updated_weight"],
		&["learning_rate", "beta_one", "beta_two", "weight_decay"],
	)?;
	let gradient = input(request, "gradient")?;
	let weight = input(request, "weight")?;
	let moment = input(request, "moment")?;
	let updated_moment = output(request, "updated_moment")?;
	let updated_weight = output(request, "updated_weight")?;
	require_equal_f32_tensors(
		request,
		weight,
		[
			("gradient", gradient),
			("moment", moment),
			("updated_moment", updated_moment),
			("updated_weight", updated_weight),
		],
	)?;
	let learning_rate = finite_nonnegative_parameter(request, "learning_rate")?;
	let beta_one = unit_interval_parameter(request, "beta_one", true)?;
	let beta_two = unit_interval_parameter(request, "beta_two", true)?;
	let weight_decay = finite_nonnegative_parameter(request, "weight_decay")?;
	let direction = emitter.intermediate(DType::F32, weight.shape.clone())?;
	emitter.emit(
		vec![moment.id, gradient.id],
		vec![updated_moment.id, direction],
		PrimitiveKind::Elementwise(Elementwise {
			program: lion_state_program(request.descriptor.id, beta_one, beta_two)?,
		}),
	)?;
	emitter.emit(
		vec![weight.id, direction],
		vec![updated_weight.id],
		PrimitiveKind::Elementwise(Elementwise {
			program: lion_weight_program(request.descriptor.id, learning_rate, weight_decay)?,
		}),
	)
}

fn emit_lamb_phase_one(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(
		request,
		&["gradient", "weight", "moment", "velocity"],
		&[
			"updated_moment",
			"updated_velocity",
			"update",
			"weight_norm_squared",
			"update_norm_squared",
		],
		&[
			"beta_one",
			"beta_two",
			"epsilon",
			"weight_decay",
			"step",
			"tree_lanes",
		],
	)?;
	let gradient = input(request, "gradient")?;
	let weight = input(request, "weight")?;
	let moment = input(request, "moment")?;
	let velocity = input(request, "velocity")?;
	let updated_moment = output(request, "updated_moment")?;
	let updated_velocity = output(request, "updated_velocity")?;
	let update = output(request, "update")?;
	let weight_norm_squared = output(request, "weight_norm_squared")?;
	let update_norm_squared = output(request, "update_norm_squared")?;
	require_equal_f32_tensors(
		request,
		weight,
		[
			("gradient", gradient),
			("moment", moment),
			("velocity", velocity),
			("updated_moment", updated_moment),
			("updated_velocity", updated_velocity),
			("update", update),
		],
	)?;
	for (name, tensor) in [
		("weight_norm_squared", weight_norm_squared),
		("update_norm_squared", update_norm_squared),
	] {
		require_dtype(request, tensor, DType::F32, name)?;
		require_shape(request, tensor, &[1], name)?;
	}
	let moments = adam_moment_parameters(request)?;
	let weight_decay = finite_nonnegative_parameter(request, "weight_decay")?;
	let tree_lanes = prepared_tree_lanes(request)?;
	let update_image = emitter.intermediate(DType::F32, weight.shape.clone())?;
	let weight_squares = emitter.intermediate(DType::F32, weight.shape.clone())?;
	let update_squares = emitter.intermediate(DType::F32, weight.shape.clone())?;
	emitter.emit(
		vec![weight.id, moment.id, velocity.id, gradient.id],
		vec![
			updated_moment.id,
			updated_velocity.id,
			update_image,
			weight_squares,
			update_squares,
		],
		PrimitiveKind::Elementwise(Elementwise {
			program: lamb_phase_one_program(request.descriptor.id, moments, weight_decay)?,
		}),
	)?;
	let reduction = sum_reduction(request, all_axes(weight)?, false, tree_lanes)?;
	emitter.emit_stage([
		KernelEmission {
			inputs: vec![weight_squares],
			outputs: vec![weight_norm_squared.id],
			kind: reduction.clone(),
		},
		KernelEmission {
			inputs: vec![update_squares],
			outputs: vec![update_norm_squared.id],
			kind: reduction,
		},
		KernelEmission {
			inputs: vec![update_image],
			outputs: vec![update.id],
			kind: PrimitiveKind::Elementwise(Elementwise {
				program: identity_program(request.descriptor.id, DType::F32)?,
			}),
		},
	])
}

fn emit_lamb_phase_two(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(
		request,
		&[
			"update",
			"weight_norm_squared",
			"update_norm_squared",
			"weight",
		],
		&["updated_weight"],
		&["learning_rate", "tree_lanes"],
	)?;
	let update = input(request, "update")?;
	let weight_norm_squared = input(request, "weight_norm_squared")?;
	let update_norm_squared = input(request, "update_norm_squared")?;
	let weight = input(request, "weight")?;
	let updated_weight = output(request, "updated_weight")?;
	require_equal_f32_tensors(
		request,
		weight,
		[("update", update), ("updated_weight", updated_weight)],
	)?;
	for (name, tensor) in [
		("weight_norm_squared", weight_norm_squared),
		("update_norm_squared", update_norm_squared),
	] {
		require_dtype(request, tensor, DType::F32, name)?;
		require_shape(request, tensor, &[1], name)?;
	}
	let learning_rate = finite_nonnegative_parameter(request, "learning_rate")?;
	let tree_lanes = prepared_tree_lanes(request)?;
	let weight_norm_image = emitter.intermediate(
		DType::F32,
		Shape::new(vec![1]).map_err(|error| language_error(request.descriptor.id, error.to_string()))?,
	)?;
	let update_norm_image = emitter.intermediate(
		DType::F32,
		Shape::new(vec![1]).map_err(|error| language_error(request.descriptor.id, error.to_string()))?,
	)?;
	let reduced_weight_norm = emitter.intermediate(
		DType::F32,
		Shape::new(vec![1]).map_err(|error| language_error(request.descriptor.id, error.to_string()))?,
	)?;
	let reduced_update_norm = emitter.intermediate(
		DType::F32,
		Shape::new(vec![1]).map_err(|error| language_error(request.descriptor.id, error.to_string()))?,
	)?;
	emitter.emit(
		vec![weight_norm_squared.id, update_norm_squared.id],
		vec![weight_norm_image, update_norm_image],
		PrimitiveKind::Elementwise(Elementwise {
			program: two_identity_program(request.descriptor.id)?,
		}),
	)?;
	let scalar_axes =
		AxisSet::new(vec![0]).map_err(|error| language_error(request.descriptor.id, error.to_string()))?;
	emitter.emit(
		vec![weight_norm_image],
		vec![reduced_weight_norm],
		sum_reduction(request, scalar_axes.clone(), false, tree_lanes)?,
	)?;
	emitter.emit(
		vec![update_norm_image],
		vec![reduced_update_norm],
		sum_reduction(request, scalar_axes, false, tree_lanes)?,
	)?;
	emitter.emit(
		vec![
			weight.id,
			update.id,
			reduced_weight_norm,
			reduced_update_norm,
		],
		vec![updated_weight.id],
		PrimitiveKind::Elementwise(Elementwise {
			program: lamb_phase_two_program(request.descriptor.id, learning_rate)?,
		}),
	)
}

fn emit_gradient_clip_norm(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	require_exact_abi(
		request,
		&["values"],
		&["clipped"],
		&["maximum_norm", "tree_lanes"],
	)?;
	let values = input(request, "values")?;
	let clipped = output(request, "clipped")?;
	require_dtype(request, values, DType::F32, "values")?;
	require_same_tensor_contract(request, values, clipped, "clipped")?;
	let maximum_norm = finite_nonnegative_parameter(request, "maximum_norm")?;
	let tree_lanes = prepared_tree_lanes(request)?;
	let squares = emitter.intermediate(DType::F32, values.shape.clone())?;
	let norm_squared = emitter.intermediate(
		DType::F32,
		Shape::new(vec![1]).map_err(|error| language_error(request.descriptor.id, error.to_string()))?,
	)?;
	emitter.emit(
		vec![values.id],
		vec![squares],
		PrimitiveKind::Elementwise(Elementwise {
			program: square_program(request.descriptor.id)?,
		}),
	)?;
	emitter.emit(
		vec![squares],
		vec![norm_squared],
		sum_reduction(request, all_axes(values)?, false, tree_lanes)?,
	)?;
	emitter.emit(
		vec![values.id, norm_squared],
		vec![clipped.id],
		PrimitiveKind::Elementwise(Elementwise {
			program: gradient_clip_program(request.descriptor.id, maximum_norm)?,
		}),
	)
}

fn emit_tree_leaf_split(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> OperationResult<()> {
	let features = input(request, "features")?;
	let feature_indices = input(request, "feature_indices")?;
	let thresholds = input(request, "thresholds")?;
	let assignments = input(request, "assignments")?;
	let destination_indices = input(request, "destination_indices")?;
	let updated = output(request, "updated_assignments")?;
	for (name, tensor, dtype) in [
		("features", features, DType::F32),
		("feature_indices", feature_indices, DType::I32),
		("thresholds", thresholds, DType::F32),
		("assignments", assignments, DType::I32),
		("destination_indices", destination_indices, DType::I32),
		("updated_assignments", updated, DType::I32),
	] {
		require_dtype(request, tensor, dtype, name)?;
	}
	if features.shape.rank() != 1 {
		return Err(request_error(
			request,
			"features must be a flattened rank-one prepared feature image",
		));
	}
	for (name, tensor) in [
		("thresholds", thresholds),
		("assignments", assignments),
		("destination_indices", destination_indices),
		("updated_assignments", updated),
	] {
		require_shape(request, tensor, feature_indices.shape.extents(), name)?;
	}
	require_true(request, "destination_indices_unique")?;
	let selected = emitter.intermediate(DType::F32, feature_indices.shape.clone())?;
	emitter.emit(
		vec![features.id, feature_indices.id],
		vec![selected],
		PrimitiveKind::Gather(Gather {
			axis: 0,
			bounds: IndexBounds::Reject,
		}),
	)?;
	let next_assignments = emitter.intermediate(DType::I32, feature_indices.shape.clone())?;
	emitter.emit(
		vec![selected, thresholds.id, assignments.id],
		vec![next_assignments],
		PrimitiveKind::Elementwise(Elementwise {
			program: tree_branch_program(request.descriptor.id)?,
		}),
	)?;
	emitter.emit(
		vec![assignments.id, destination_indices.id, next_assignments],
		vec![updated.id],
		PrimitiveKind::Scatter(Scatter {
			axis: 0,
			bounds: IndexBounds::Reject,
			conflict: ScatterConflict::UniqueIndices,
		}),
	)
}

fn fft_linear_stage_program(operation: OperationId) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let left_real = scalar_input(operation, &mut builder, DType::F32)?;
	let left_imaginary = scalar_input(operation, &mut builder, DType::F32)?;
	let right_real = scalar_input(operation, &mut builder, DType::F32)?;
	let right_imaginary = scalar_input(operation, &mut builder, DType::F32)?;
	let real_from_left_real = scalar_input(operation, &mut builder, DType::F32)?;
	let real_from_left_imaginary = scalar_input(operation, &mut builder, DType::F32)?;
	let real_from_right_real = scalar_input(operation, &mut builder, DType::F32)?;
	let real_from_right_imaginary = scalar_input(operation, &mut builder, DType::F32)?;
	let imaginary_from_left_real = scalar_input(operation, &mut builder, DType::F32)?;
	let imaginary_from_left_imaginary = scalar_input(operation, &mut builder, DType::F32)?;
	let imaginary_from_right_real = scalar_input(operation, &mut builder, DType::F32)?;
	let imaginary_from_right_imaginary = scalar_input(operation, &mut builder, DType::F32)?;
	let real = weighted_sum_four(
		operation,
		&mut builder,
		[
			(left_real, real_from_left_real),
			(left_imaginary, real_from_left_imaginary),
			(right_real, real_from_right_real),
			(right_imaginary, real_from_right_imaginary),
		],
	)?;
	let imaginary = weighted_sum_four(
		operation,
		&mut builder,
		[
			(left_real, imaginary_from_left_real),
			(left_imaginary, imaginary_from_left_imaginary),
			(right_real, imaginary_from_right_real),
			(right_imaginary, imaginary_from_right_imaginary),
		],
	)?;
	scalar_finish(operation, builder, &[real, imaginary])
}

fn weighted_sum_four(
	operation: OperationId,
	builder: &mut ScalarProgramBuilder,
	terms: [(
		recipe_language::ScalarExpression,
		recipe_language::ScalarExpression,
	); 4],
) -> OperationResult<recipe_language::ScalarExpression> {
	let mut result = scalar_binary(
		operation,
		builder,
		ScalarOpcode::Multiply,
		terms[0].0,
		terms[0].1,
	)?;
	for (value, coefficient) in terms.into_iter().skip(1) {
		result = builder
			.ternary(ScalarOpcode::Fma, value, coefficient, result)
			.map_err(|error| language_error(operation, error.to_string()))?;
	}
	Ok(result)
}

fn multiply_program(operation: OperationId) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let left = scalar_input(operation, &mut builder, DType::F32)?;
	let right = scalar_input(operation, &mut builder, DType::F32)?;
	let output = scalar_binary(operation, &mut builder, ScalarOpcode::Multiply, left, right)?;
	scalar_finish(operation, builder, &[output])
}

fn triangular_solve_program(operation: OperationId) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let rhs = scalar_input(operation, &mut builder, DType::F32)?;
	let prefix = scalar_input(operation, &mut builder, DType::F32)?;
	let diagonal = scalar_input(operation, &mut builder, DType::F32)?;
	let zero = builder
		.f32(0.0)
		.map_err(|error| language_error(operation, error.to_string()))?;
	let nonzero = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::NotEqual,
		diagonal,
		zero,
	)?;
	builder
		.unary(ScalarOpcode::Require, nonzero)
		.map_err(|error| language_error(operation, error.to_string()))?;
	let residual = scalar_binary(operation, &mut builder, ScalarOpcode::Subtract, rhs, prefix)?;
	let output = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Divide,
		residual,
		diagonal,
	)?;
	scalar_finish(operation, builder, &[output])
}

fn identity_program(operation: OperationId, dtype: DType) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let value = scalar_input(operation, &mut builder, dtype)?;
	scalar_finish(operation, builder, &[value])
}

fn batch_normalization_program(operation: OperationId, epsilon: f32) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let value = scalar_input(operation, &mut builder, DType::F32)?;
	let mean = scalar_input(operation, &mut builder, DType::F32)?;
	let variance = scalar_input(operation, &mut builder, DType::F32)?;
	let scale = scalar_input(operation, &mut builder, DType::F32)?;
	let bias = scalar_input(operation, &mut builder, DType::F32)?;
	let epsilon = builder
		.f32(epsilon)
		.map_err(|error| language_error(operation, error.to_string()))?;
	let adjusted_variance = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Add,
		variance,
		epsilon,
	)?;
	let zero = builder
		.f32(0.0)
		.map_err(|error| language_error(operation, error.to_string()))?;
	let positive = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::GreaterThan,
		adjusted_variance,
		zero,
	)?;
	builder
		.unary(ScalarOpcode::Require, positive)
		.map_err(|error| language_error(operation, error.to_string()))?;
	let standard_deviation = builder
		.unary(ScalarOpcode::SquareRoot, adjusted_variance)
		.map_err(|error| language_error(operation, error.to_string()))?;
	let centered = scalar_binary(operation, &mut builder, ScalarOpcode::Subtract, value, mean)?;
	let scaled = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		scale,
		centered,
	)?;
	let standardized = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Divide,
		scaled,
		standard_deviation,
	)?;
	let output = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Add,
		standardized,
		bias,
	)?;
	scalar_finish(operation, builder, &[output])
}

fn centered_square_program(operation: OperationId, divisor: f32) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let value = scalar_input(operation, &mut builder, DType::F32)?;
	let sum = scalar_input(operation, &mut builder, DType::F32)?;
	let divisor = scalar_f32(operation, &mut builder, divisor)?;
	let mean = scalar_binary(operation, &mut builder, ScalarOpcode::Divide, sum, divisor)?;
	let centered = scalar_binary(operation, &mut builder, ScalarOpcode::Subtract, value, mean)?;
	let squared = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		centered,
		centered,
	)?;
	scalar_finish(operation, builder, &[squared])
}

fn normalization_statistics_program(
	operation: OperationId,
	divisor: f32,
	epsilon: f32,
) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let sum = scalar_input(operation, &mut builder, DType::F32)?;
	let variance_sum = scalar_input(operation, &mut builder, DType::F32)?;
	let divisor = scalar_f32(operation, &mut builder, divisor)?;
	let mean = scalar_binary(operation, &mut builder, ScalarOpcode::Divide, sum, divisor)?;
	let inverse = inverse_standard_deviation(operation, &mut builder, variance_sum, divisor, epsilon)?;
	scalar_finish(operation, builder, &[mean, inverse])
}

fn affine_normalization_program(operation: OperationId) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let value = scalar_input(operation, &mut builder, DType::F32)?;
	let scale = scalar_input(operation, &mut builder, DType::F32)?;
	let bias = scalar_input(operation, &mut builder, DType::F32)?;
	let mean = scalar_input(operation, &mut builder, DType::F32)?;
	let inverse = scalar_input(operation, &mut builder, DType::F32)?;
	let centered = scalar_binary(operation, &mut builder, ScalarOpcode::Subtract, value, mean)?;
	let scaled = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		scale,
		centered,
	)?;
	let scaled = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		scaled,
		inverse,
	)?;
	let normalized = scalar_binary(operation, &mut builder, ScalarOpcode::Add, scaled, bias)?;
	scalar_finish(operation, builder, &[normalized])
}

fn batch_normalization_backward_terms_program(operation: OperationId) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let output_gradient = scalar_input(operation, &mut builder, DType::F32)?;
	let value = scalar_input(operation, &mut builder, DType::F32)?;
	let mean = scalar_input(operation, &mut builder, DType::F32)?;
	let inverse = scalar_input(operation, &mut builder, DType::F32)?;
	let centered = scalar_binary(operation, &mut builder, ScalarOpcode::Subtract, value, mean)?;
	let normalized = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		centered,
		inverse,
	)?;
	let normalized_gradient = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		output_gradient,
		normalized,
	)?;
	scalar_finish(operation, builder, &[output_gradient, normalized_gradient])
}

fn batch_normalization_gradient_program(
	operation: OperationId,
	divisor: f32,
) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let output_gradient = scalar_input(operation, &mut builder, DType::F32)?;
	let value = scalar_input(operation, &mut builder, DType::F32)?;
	let mean = scalar_input(operation, &mut builder, DType::F32)?;
	let inverse = scalar_input(operation, &mut builder, DType::F32)?;
	let scale = scalar_input(operation, &mut builder, DType::F32)?;
	let output_gradient_sum = scalar_input(operation, &mut builder, DType::F32)?;
	let normalized_gradient_sum = scalar_input(operation, &mut builder, DType::F32)?;
	let centered = scalar_binary(operation, &mut builder, ScalarOpcode::Subtract, value, mean)?;
	let normalized = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		centered,
		inverse,
	)?;
	let inverse_divisor = scalar_f32(operation, &mut builder, 1.0 / divisor)?;
	let mean_gradient = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		output_gradient_sum,
		inverse_divisor,
	)?;
	let normalized_mean_gradient = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		normalized,
		normalized_gradient_sum,
	)?;
	let normalized_mean_gradient = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		normalized_mean_gradient,
		inverse_divisor,
	)?;
	let residual = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Subtract,
		output_gradient,
		mean_gradient,
	)?;
	let residual = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Subtract,
		residual,
		normalized_mean_gradient,
	)?;
	let prefix = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		scale,
		inverse,
	)?;
	let input_gradient = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		prefix,
		residual,
	)?;
	let scale_gradient = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		output_gradient,
		normalized,
	)?;
	scalar_finish(
		operation,
		builder,
		&[input_gradient, scale_gradient, output_gradient],
	)
}

fn running_statistics_update_program(
	operation: OperationId,
	momentum: f32,
) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let running_mean = scalar_input(operation, &mut builder, DType::F32)?;
	let running_variance = scalar_input(operation, &mut builder, DType::F32)?;
	let saved_mean = scalar_input(operation, &mut builder, DType::F32)?;
	let saved_variance = scalar_input(operation, &mut builder, DType::F32)?;
	let momentum = scalar_f32(operation, &mut builder, momentum)?;
	let one = scalar_f32(operation, &mut builder, 1.0)?;
	let retained = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Subtract,
		one,
		momentum,
	)?;
	let retained_mean = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		retained,
		running_mean,
	)?;
	let fresh_mean = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		momentum,
		saved_mean,
	)?;
	let updated_mean = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Add,
		retained_mean,
		fresh_mean,
	)?;
	let retained_variance = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		retained,
		running_variance,
	)?;
	let fresh_variance = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		momentum,
		saved_variance,
	)?;
	let updated_variance = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Add,
		retained_variance,
		fresh_variance,
	)?;
	scalar_finish(operation, builder, &[updated_mean, updated_variance])
}

fn layer_normalization_program(
	operation: OperationId,
	divisor: f32,
	epsilon: f32,
	has_scale: bool,
	has_bias: bool,
) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let value = scalar_input(operation, &mut builder, DType::F32)?;
	let sum = scalar_input(operation, &mut builder, DType::F32)?;
	let variance_sum = scalar_input(operation, &mut builder, DType::F32)?;
	let scale = match has_scale {
		true => Some(scalar_input(operation, &mut builder, DType::F32)?),
		false => None,
	};
	let bias = match has_bias {
		true => Some(scalar_input(operation, &mut builder, DType::F32)?),
		false => None,
	};
	let divisor = scalar_f32(operation, &mut builder, divisor)?;
	let mean = scalar_binary(operation, &mut builder, ScalarOpcode::Divide, sum, divisor)?;
	let inverse = inverse_standard_deviation(operation, &mut builder, variance_sum, divisor, epsilon)?;
	let centered = scalar_binary(operation, &mut builder, ScalarOpcode::Subtract, value, mean)?;
	let normalized = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		centered,
		inverse,
	)?;
	let normalized = match scale {
		Some(scale) => {
			let scaled = scalar_binary(
				operation,
				&mut builder,
				ScalarOpcode::Multiply,
				normalized,
				scale,
			)?;
			match bias {
				Some(bias) => scalar_binary(operation, &mut builder, ScalarOpcode::Add, scaled, bias)?,
				None => scaled,
			}
		}
		None => normalized,
	};
	scalar_finish(operation, builder, &[normalized])
}

fn layer_normalization_backward_terms_program(
	operation: OperationId,
	divisor: f32,
	epsilon: f32,
) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let output_gradient = scalar_input(operation, &mut builder, DType::F32)?;
	let value = scalar_input(operation, &mut builder, DType::F32)?;
	let scale = scalar_input(operation, &mut builder, DType::F32)?;
	let sum = scalar_input(operation, &mut builder, DType::F32)?;
	let variance_sum = scalar_input(operation, &mut builder, DType::F32)?;
	let divisor = scalar_f32(operation, &mut builder, divisor)?;
	let mean = scalar_binary(operation, &mut builder, ScalarOpcode::Divide, sum, divisor)?;
	let inverse = inverse_standard_deviation(operation, &mut builder, variance_sum, divisor, epsilon)?;
	let centered = scalar_binary(operation, &mut builder, ScalarOpcode::Subtract, value, mean)?;
	let normalized = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		centered,
		inverse,
	)?;
	let first_term = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		output_gradient,
		scale,
	)?;
	let second_term = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		first_term,
		normalized,
	)?;
	let scale_gradient = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		output_gradient,
		normalized,
	)?;
	scalar_finish(
		operation,
		builder,
		&[first_term, second_term, scale_gradient, output_gradient],
	)
}

fn layer_normalization_input_gradient_program(
	operation: OperationId,
	divisor: f32,
	epsilon: f32,
) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let output_gradient = scalar_input(operation, &mut builder, DType::F32)?;
	let value = scalar_input(operation, &mut builder, DType::F32)?;
	let scale = scalar_input(operation, &mut builder, DType::F32)?;
	let sum = scalar_input(operation, &mut builder, DType::F32)?;
	let variance_sum = scalar_input(operation, &mut builder, DType::F32)?;
	let first_sum = scalar_input(operation, &mut builder, DType::F32)?;
	let second_sum = scalar_input(operation, &mut builder, DType::F32)?;
	let divisor_expression = scalar_f32(operation, &mut builder, divisor)?;
	let mean = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Divide,
		sum,
		divisor_expression,
	)?;
	let inverse = inverse_standard_deviation(
		operation,
		&mut builder,
		variance_sum,
		divisor_expression,
		epsilon,
	)?;
	let centered = scalar_binary(operation, &mut builder, ScalarOpcode::Subtract, value, mean)?;
	let normalized = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		centered,
		inverse,
	)?;
	let transformed_gradient = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		output_gradient,
		scale,
	)?;
	let inverse_divisor = scalar_f32(operation, &mut builder, 1.0 / divisor)?;
	let first_mean = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		first_sum,
		inverse_divisor,
	)?;
	let second_mean = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		normalized,
		second_sum,
	)?;
	let second_mean = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		second_mean,
		inverse_divisor,
	)?;
	let residual = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Subtract,
		transformed_gradient,
		first_mean,
	)?;
	let residual = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Subtract,
		residual,
		second_mean,
	)?;
	let input_gradient = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		inverse,
		residual,
	)?;
	scalar_finish(operation, builder, &[input_gradient])
}

fn rms_normalization_program(
	operation: OperationId,
	divisor: f32,
	epsilon: f32,
	has_scale: bool,
) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let value = scalar_input(operation, &mut builder, DType::F32)?;
	let square_sum = scalar_input(operation, &mut builder, DType::F32)?;
	let scale = match has_scale {
		true => Some(scalar_input(operation, &mut builder, DType::F32)?),
		false => None,
	};
	let divisor = scalar_f32(operation, &mut builder, divisor)?;
	let inverse = inverse_standard_deviation(operation, &mut builder, square_sum, divisor, epsilon)?;
	let normalized = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		value,
		inverse,
	)?;
	let normalized = match scale {
		Some(scale) => scalar_binary(
			operation,
			&mut builder,
			ScalarOpcode::Multiply,
			normalized,
			scale,
		)?,
		None => normalized,
	};
	scalar_finish(operation, builder, &[normalized])
}

fn rms_dot_term_program(
	operation: OperationId,
	divisor: f32,
	epsilon: f32,
) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let output_gradient = scalar_input(operation, &mut builder, DType::F32)?;
	let value = scalar_input(operation, &mut builder, DType::F32)?;
	let square_sum = scalar_input(operation, &mut builder, DType::F32)?;
	let divisor = scalar_f32(operation, &mut builder, divisor)?;
	let inverse = inverse_standard_deviation(operation, &mut builder, square_sum, divisor, epsilon)?;
	let term = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		output_gradient,
		value,
	)?;
	let term = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		term,
		inverse,
	)?;
	scalar_finish(operation, builder, &[term])
}

fn rms_normalization_backward_program(
	operation: OperationId,
	divisor: f32,
	epsilon: f32,
) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let output_gradient = scalar_input(operation, &mut builder, DType::F32)?;
	let value = scalar_input(operation, &mut builder, DType::F32)?;
	let scale = scalar_input(operation, &mut builder, DType::F32)?;
	let square_sum = scalar_input(operation, &mut builder, DType::F32)?;
	let dot_sum = scalar_input(operation, &mut builder, DType::F32)?;
	let divisor_expression = scalar_f32(operation, &mut builder, divisor)?;
	let inverse = inverse_standard_deviation(
		operation,
		&mut builder,
		square_sum,
		divisor_expression,
		epsilon,
	)?;
	let normalized = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		value,
		inverse,
	)?;
	let dot = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Divide,
		dot_sum,
		divisor_expression,
	)?;
	let projected = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		normalized,
		dot,
	)?;
	let residual = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Subtract,
		output_gradient,
		projected,
	)?;
	let prefix = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		scale,
		inverse,
	)?;
	let input_gradient = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		prefix,
		residual,
	)?;
	let scale_gradient = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		output_gradient,
		normalized,
	)?;
	scalar_finish(operation, builder, &[input_gradient, scale_gradient])
}

fn inverse_standard_deviation(
	operation: OperationId,
	builder: &mut ScalarProgramBuilder,
	sum: recipe_language::ScalarExpression,
	divisor: recipe_language::ScalarExpression,
	epsilon: f32,
) -> OperationResult<recipe_language::ScalarExpression> {
	let mean = scalar_binary(operation, builder, ScalarOpcode::Divide, sum, divisor)?;
	let epsilon = scalar_f32(operation, builder, epsilon)?;
	let adjusted = scalar_binary(operation, builder, ScalarOpcode::Add, mean, epsilon)?;
	require_positive(operation, builder, adjusted)?;
	let root = scalar_unary(operation, builder, ScalarOpcode::SquareRoot, adjusted)?;
	let one = scalar_f32(operation, builder, 1.0)?;
	scalar_binary(operation, builder, ScalarOpcode::Divide, one, root)
}

fn momentum_program(
	operation: OperationId,
	momentum: f32,
	learning_rate: f32,
) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let velocity = scalar_input(operation, &mut builder, DType::F32)?;
	let gradient = scalar_input(operation, &mut builder, DType::F32)?;
	let momentum = builder
		.f32(momentum)
		.map_err(|error| language_error(operation, error.to_string()))?;
	let carried = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		momentum,
		velocity,
	)?;
	let learning_rate = scalar_f32(operation, &mut builder, learning_rate)?;
	let gradient_step = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		learning_rate,
		gradient,
	)?;
	let output = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Subtract,
		carried,
		gradient_step,
	)?;
	scalar_finish(operation, builder, &[output])
}

fn momentum_weight_program(operation: OperationId) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let parameter = scalar_input(operation, &mut builder, DType::F32)?;
	let velocity = scalar_input(operation, &mut builder, DType::F32)?;
	let output = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Add,
		parameter,
		velocity,
	)?;
	scalar_finish(operation, builder, &[output])
}

fn adagrad_state_program(operation: OperationId) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let accumulator = scalar_input(operation, &mut builder, DType::F32)?;
	let gradient = scalar_input(operation, &mut builder, DType::F32)?;
	let gradient_squared = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		gradient,
		gradient,
	)?;
	let updated = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Add,
		accumulator,
		gradient_squared,
	)?;
	scalar_finish(operation, builder, &[updated])
}

fn rmsprop_state_program(operation: OperationId, decay: f32) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let cache = scalar_input(operation, &mut builder, DType::F32)?;
	let gradient = scalar_input(operation, &mut builder, DType::F32)?;
	let decay = scalar_f32(operation, &mut builder, decay)?;
	let one = scalar_f32(operation, &mut builder, 1.0)?;
	let complement = scalar_binary(operation, &mut builder, ScalarOpcode::Subtract, one, decay)?;
	let carried = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		decay,
		cache,
	)?;
	let fresh = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		complement,
		gradient,
	)?;
	let fresh = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		fresh,
		gradient,
	)?;
	let updated = scalar_binary(operation, &mut builder, ScalarOpcode::Add, carried, fresh)?;
	scalar_finish(operation, builder, &[updated])
}

fn adaptive_weight_program(
	operation: OperationId,
	learning_rate: f32,
	epsilon: f32,
) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let weight = scalar_input(operation, &mut builder, DType::F32)?;
	let gradient = scalar_input(operation, &mut builder, DType::F32)?;
	let state = scalar_input(operation, &mut builder, DType::F32)?;
	require_nonnegative(operation, &mut builder, state)?;
	let root = scalar_unary(operation, &mut builder, ScalarOpcode::SquareRoot, state)?;
	let epsilon = scalar_f32(operation, &mut builder, epsilon)?;
	let denominator = scalar_binary(operation, &mut builder, ScalarOpcode::Add, root, epsilon)?;
	require_positive(operation, &mut builder, denominator)?;
	let learning_rate = scalar_f32(operation, &mut builder, learning_rate)?;
	let numerator = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		learning_rate,
		gradient,
	)?;
	let step = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Divide,
		numerator,
		denominator,
	)?;
	let updated = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Subtract,
		weight,
		step,
	)?;
	scalar_finish(operation, builder, &[updated])
}

fn adam_state_program(
	operation: OperationId,
	beta_one: f32,
	beta_two: f32,
) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let first_moment = scalar_input(operation, &mut builder, DType::F32)?;
	let second_moment = scalar_input(operation, &mut builder, DType::F32)?;
	let gradient = scalar_input(operation, &mut builder, DType::F32)?;
	let beta_one = scalar_f32(operation, &mut builder, beta_one)?;
	let beta_two = scalar_f32(operation, &mut builder, beta_two)?;
	let one = scalar_f32(operation, &mut builder, 1.0)?;
	let one_minus_beta_one = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Subtract,
		one,
		beta_one,
	)?;
	let one_minus_beta_two = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Subtract,
		one,
		beta_two,
	)?;
	let carried_first = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		beta_one,
		first_moment,
	)?;
	let fresh_first = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		one_minus_beta_one,
		gradient,
	)?;
	let updated_first = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Add,
		carried_first,
		fresh_first,
	)?;
	let carried_second = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		beta_two,
		second_moment,
	)?;
	let fresh_second = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		one_minus_beta_two,
		gradient,
	)?;
	let fresh_second = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		fresh_second,
		gradient,
	)?;
	let updated_second = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Add,
		carried_second,
		fresh_second,
	)?;
	scalar_finish(operation, builder, &[updated_first, updated_second])
}

fn adam_weight_program(
	operation: OperationId,
	parameters: AdamMomentParameters,
	learning_rate: f32,
	weight_decay: f32,
	decoupled_weight_decay: bool,
) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let weight = scalar_input(operation, &mut builder, DType::F32)?;
	let first_moment = scalar_input(operation, &mut builder, DType::F32)?;
	let second_moment = scalar_input(operation, &mut builder, DType::F32)?;
	let corrected_first = bias_correct(
		operation,
		&mut builder,
		first_moment,
		parameters.first_correction,
	)?;
	let corrected_second = bias_correct(
		operation,
		&mut builder,
		second_moment,
		parameters.second_correction,
	)?;
	require_nonnegative(operation, &mut builder, corrected_second)?;
	let root = scalar_unary(
		operation,
		&mut builder,
		ScalarOpcode::SquareRoot,
		corrected_second,
	)?;
	let epsilon = scalar_f32(operation, &mut builder, parameters.epsilon)?;
	let denominator = scalar_binary(operation, &mut builder, ScalarOpcode::Add, root, epsilon)?;
	require_positive(operation, &mut builder, denominator)?;
	let learning_rate = scalar_f32(operation, &mut builder, learning_rate)?;
	let numerator = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		learning_rate,
		corrected_first,
	)?;
	let adaptive_step = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Divide,
		numerator,
		denominator,
	)?;
	let base = match decoupled_weight_decay {
		true => {
			let weight_decay = scalar_f32(operation, &mut builder, weight_decay)?;
			let decay = scalar_binary(
				operation,
				&mut builder,
				ScalarOpcode::Multiply,
				learning_rate,
				weight_decay,
			)?;
			let one = scalar_f32(operation, &mut builder, 1.0)?;
			let factor = scalar_binary(operation, &mut builder, ScalarOpcode::Subtract, one, decay)?;
			scalar_binary(
				operation,
				&mut builder,
				ScalarOpcode::Multiply,
				weight,
				factor,
			)?
		}
		false => weight,
	};
	let updated = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Subtract,
		base,
		adaptive_step,
	)?;
	scalar_finish(operation, builder, &[updated])
}

fn nadam_weight_program(
	operation: OperationId,
	parameters: AdamMomentParameters,
	learning_rate: f32,
) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let weight = scalar_input(operation, &mut builder, DType::F32)?;
	let gradient = scalar_input(operation, &mut builder, DType::F32)?;
	let first_moment = scalar_input(operation, &mut builder, DType::F32)?;
	let second_moment = scalar_input(operation, &mut builder, DType::F32)?;
	let beta_one = scalar_f32(operation, &mut builder, parameters.beta_one)?;
	let one = scalar_f32(operation, &mut builder, 1.0)?;
	let one_minus_beta_one = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Subtract,
		one,
		beta_one,
	)?;
	let carried = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		beta_one,
		first_moment,
	)?;
	let next_correction = scalar_f32(operation, &mut builder, parameters.next_first_correction)?;
	let carried = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Divide,
		carried,
		next_correction,
	)?;
	let fresh = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		one_minus_beta_one,
		gradient,
	)?;
	let first_correction = scalar_f32(operation, &mut builder, parameters.first_correction)?;
	let fresh = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Divide,
		fresh,
		first_correction,
	)?;
	let corrected_first = scalar_binary(operation, &mut builder, ScalarOpcode::Add, carried, fresh)?;
	let corrected_second = bias_correct(
		operation,
		&mut builder,
		second_moment,
		parameters.second_correction,
	)?;
	require_nonnegative(operation, &mut builder, corrected_second)?;
	let root = scalar_unary(
		operation,
		&mut builder,
		ScalarOpcode::SquareRoot,
		corrected_second,
	)?;
	let epsilon = scalar_f32(operation, &mut builder, parameters.epsilon)?;
	let denominator = scalar_binary(operation, &mut builder, ScalarOpcode::Add, root, epsilon)?;
	require_positive(operation, &mut builder, denominator)?;
	let learning_rate = scalar_f32(operation, &mut builder, learning_rate)?;
	let numerator = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		learning_rate,
		corrected_first,
	)?;
	let step = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Divide,
		numerator,
		denominator,
	)?;
	let updated = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Subtract,
		weight,
		step,
	)?;
	scalar_finish(operation, builder, &[updated])
}

fn lion_state_program(
	operation: OperationId,
	beta_one: f32,
	beta_two: f32,
) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let moment = scalar_input(operation, &mut builder, DType::F32)?;
	let gradient = scalar_input(operation, &mut builder, DType::F32)?;
	let one = scalar_f32(operation, &mut builder, 1.0)?;
	let beta_one = scalar_f32(operation, &mut builder, beta_one)?;
	let beta_two = scalar_f32(operation, &mut builder, beta_two)?;
	let one_minus_beta_one = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Subtract,
		one,
		beta_one,
	)?;
	let one_minus_beta_two = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Subtract,
		one,
		beta_two,
	)?;
	let carried_direction = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		beta_one,
		moment,
	)?;
	let fresh_direction = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		one_minus_beta_one,
		gradient,
	)?;
	let interpolation = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Add,
		carried_direction,
		fresh_direction,
	)?;
	let zero = scalar_f32(operation, &mut builder, 0.0)?;
	let positive = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::GreaterThan,
		interpolation,
		zero,
	)?;
	let negative = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::LessThan,
		interpolation,
		zero,
	)?;
	let minus_one = scalar_f32(operation, &mut builder, -1.0)?;
	let negative_or_zero = scalar_ternary(
		operation,
		&mut builder,
		ScalarOpcode::Select,
		negative,
		minus_one,
		zero,
	)?;
	let direction = scalar_ternary(
		operation,
		&mut builder,
		ScalarOpcode::Select,
		positive,
		one,
		negative_or_zero,
	)?;
	let carried_moment = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		beta_two,
		moment,
	)?;
	let fresh_moment = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		one_minus_beta_two,
		gradient,
	)?;
	let updated_moment = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Add,
		carried_moment,
		fresh_moment,
	)?;
	scalar_finish(operation, builder, &[updated_moment, direction])
}

fn lion_weight_program(
	operation: OperationId,
	learning_rate: f32,
	weight_decay: f32,
) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let weight = scalar_input(operation, &mut builder, DType::F32)?;
	let direction = scalar_input(operation, &mut builder, DType::F32)?;
	let weight_decay = scalar_f32(operation, &mut builder, weight_decay)?;
	let decay = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		weight_decay,
		weight,
	)?;
	let update = scalar_binary(operation, &mut builder, ScalarOpcode::Add, direction, decay)?;
	let learning_rate = scalar_f32(operation, &mut builder, learning_rate)?;
	let step = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		learning_rate,
		update,
	)?;
	let updated = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Subtract,
		weight,
		step,
	)?;
	scalar_finish(operation, builder, &[updated])
}

fn lamb_phase_one_program(
	operation: OperationId,
	parameters: AdamMomentParameters,
	weight_decay: f32,
) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let weight = scalar_input(operation, &mut builder, DType::F32)?;
	let moment = scalar_input(operation, &mut builder, DType::F32)?;
	let velocity = scalar_input(operation, &mut builder, DType::F32)?;
	let gradient = scalar_input(operation, &mut builder, DType::F32)?;
	let beta_one = scalar_f32(operation, &mut builder, parameters.beta_one)?;
	let beta_two = scalar_f32(operation, &mut builder, parameters.beta_two)?;
	let one = scalar_f32(operation, &mut builder, 1.0)?;
	let one_minus_beta_one = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Subtract,
		one,
		beta_one,
	)?;
	let one_minus_beta_two = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Subtract,
		one,
		beta_two,
	)?;
	let carried_moment = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		beta_one,
		moment,
	)?;
	let fresh_moment = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		one_minus_beta_one,
		gradient,
	)?;
	let updated_moment = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Add,
		carried_moment,
		fresh_moment,
	)?;
	let carried_velocity = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		beta_two,
		velocity,
	)?;
	let fresh_velocity = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		one_minus_beta_two,
		gradient,
	)?;
	let fresh_velocity = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		fresh_velocity,
		gradient,
	)?;
	let updated_velocity = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Add,
		carried_velocity,
		fresh_velocity,
	)?;
	let corrected_moment = bias_correct(
		operation,
		&mut builder,
		updated_moment,
		parameters.first_correction,
	)?;
	let corrected_velocity = bias_correct(
		operation,
		&mut builder,
		updated_velocity,
		parameters.second_correction,
	)?;
	require_nonnegative(operation, &mut builder, corrected_velocity)?;
	let root = scalar_unary(
		operation,
		&mut builder,
		ScalarOpcode::SquareRoot,
		corrected_velocity,
	)?;
	let epsilon = scalar_f32(operation, &mut builder, parameters.epsilon)?;
	let denominator = scalar_binary(operation, &mut builder, ScalarOpcode::Add, root, epsilon)?;
	require_positive(operation, &mut builder, denominator)?;
	let adaptive = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Divide,
		corrected_moment,
		denominator,
	)?;
	let weight_decay = scalar_f32(operation, &mut builder, weight_decay)?;
	let decay = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		weight_decay,
		weight,
	)?;
	let update = scalar_binary(operation, &mut builder, ScalarOpcode::Add, adaptive, decay)?;
	let weight_squared = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		weight,
		weight,
	)?;
	let update_squared = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		update,
		update,
	)?;
	scalar_finish(
		operation,
		builder,
		&[
			updated_moment,
			updated_velocity,
			update,
			weight_squared,
			update_squared,
		],
	)
}

fn lamb_phase_two_program(operation: OperationId, learning_rate: f32) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let weight = scalar_input(operation, &mut builder, DType::F32)?;
	let update = scalar_input(operation, &mut builder, DType::F32)?;
	let weight_norm_squared = scalar_input(operation, &mut builder, DType::F32)?;
	let update_norm_squared = scalar_input(operation, &mut builder, DType::F32)?;
	require_nonnegative(operation, &mut builder, weight_norm_squared)?;
	require_nonnegative(operation, &mut builder, update_norm_squared)?;
	let weight_norm = scalar_unary(
		operation,
		&mut builder,
		ScalarOpcode::SquareRoot,
		weight_norm_squared,
	)?;
	let update_norm = scalar_unary(
		operation,
		&mut builder,
		ScalarOpcode::SquareRoot,
		update_norm_squared,
	)?;
	let zero = scalar_f32(operation, &mut builder, 0.0)?;
	let weight_zero = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Equal,
		weight_norm,
		zero,
	)?;
	let update_zero = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Equal,
		update_norm,
		zero,
	)?;
	let either_zero = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::BitOr,
		weight_zero,
		update_zero,
	)?;
	let ratio = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Divide,
		weight_norm,
		update_norm,
	)?;
	let one = scalar_f32(operation, &mut builder, 1.0)?;
	let ratio = scalar_ternary(
		operation,
		&mut builder,
		ScalarOpcode::Select,
		either_zero,
		one,
		ratio,
	)?;
	let learning_rate = scalar_f32(operation, &mut builder, learning_rate)?;
	let step = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		learning_rate,
		ratio,
	)?;
	let step = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		step,
		update,
	)?;
	let updated = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Subtract,
		weight,
		step,
	)?;
	scalar_finish(operation, builder, &[updated])
}

fn two_identity_program(operation: OperationId) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let first = scalar_input(operation, &mut builder, DType::F32)?;
	let second = scalar_input(operation, &mut builder, DType::F32)?;
	scalar_finish(operation, builder, &[first, second])
}

fn square_program(operation: OperationId) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let value = scalar_input(operation, &mut builder, DType::F32)?;
	let squared = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		value,
		value,
	)?;
	scalar_finish(operation, builder, &[squared])
}

fn gradient_clip_program(operation: OperationId, maximum_norm: f32) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let value = scalar_input(operation, &mut builder, DType::F32)?;
	let norm_squared = scalar_input(operation, &mut builder, DType::F32)?;
	require_nonnegative(operation, &mut builder, norm_squared)?;
	let norm = scalar_unary(
		operation,
		&mut builder,
		ScalarOpcode::SquareRoot,
		norm_squared,
	)?;
	let maximum_norm = scalar_f32(operation, &mut builder, maximum_norm)?;
	let within = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::LessThanOrEqual,
		norm,
		maximum_norm,
	)?;
	let scale = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Divide,
		maximum_norm,
		norm,
	)?;
	let scaled = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		value,
		scale,
	)?;
	let clipped = scalar_ternary(
		operation,
		&mut builder,
		ScalarOpcode::Select,
		within,
		value,
		scaled,
	)?;
	scalar_finish(operation, builder, &[clipped])
}

fn tree_branch_program(operation: OperationId) -> OperationResult<recipe_core::ScalarProgram> {
	let mut builder = scalar_builder(operation)?;
	let feature = scalar_input(operation, &mut builder, DType::F32)?;
	let threshold = scalar_input(operation, &mut builder, DType::F32)?;
	let assignment = scalar_input(operation, &mut builder, DType::I32)?;
	let branch = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::GreaterThan,
		feature,
		threshold,
	)?;
	let two = builder
		.i32(2)
		.map_err(|error| language_error(operation, error.to_string()))?;
	let doubled = scalar_binary(
		operation,
		&mut builder,
		ScalarOpcode::Multiply,
		assignment,
		two,
	)?;
	let output = scalar_binary(operation, &mut builder, ScalarOpcode::Add, doubled, branch)?;
	scalar_finish(operation, builder, &[output])
}

#[derive(Clone, Copy, Debug)]
struct AdamMomentParameters {
	beta_one: f32,
	beta_two: f32,
	epsilon: f32,
	first_correction: f32,
	next_first_correction: f32,
	second_correction: f32,
}

fn normalization_matrix(
	request: &MaterializationRequest<'_>,
	tensor: &Tensor,
	name: &str,
) -> OperationResult<(u64, u64, f32, f32)> {
	require_dtype(request, tensor, DType::F32, name)?;
	if tensor.shape.rank() != 2 {
		return Err(operation_error(
			request.descriptor.id,
			OperationErrorKind::UnsupportedConcreteShape,
			format!("{name} must be a rank-two [rows, columns] tensor"),
		));
	}
	let rows = tensor.shape.extents()[0];
	let columns = tensor.shape.extents()[1];
	if rows == 0 || columns == 0 || rows > MAX_I32_INDEX || columns > MAX_I32_INDEX {
		return Err(operation_error(
			request.descriptor.id,
			OperationErrorKind::UnsupportedConcreteShape,
			format!("{name} dimensions must each be in 1..={MAX_I32_INDEX}"),
		));
	}
	let row_divisor = exact_f32_extent(request, rows, "rows")?;
	let column_divisor = exact_f32_extent(request, columns, "columns")?;
	Ok((rows, columns, row_divisor, column_divisor))
}

fn exact_f32_extent(request: &MaterializationRequest<'_>, value: u64, name: &str) -> OperationResult<f32> {
	const MAX_EXACT_F32_INTEGER: u64 = 16_777_216;
	if value > MAX_EXACT_F32_INTEGER {
		return Err(operation_error(
			request.descriptor.id,
			OperationErrorKind::UnsupportedConcreteShape,
			format!("{name} extent {value} is not guaranteed lossless in f32"),
		));
	}
	let high = u16::try_from(value >> 16).map_err(|error| {
		request_error(
			request,
			format!("{name} high word does not fit u16: {error}"),
		)
	})?;
	let low = u16::try_from(value & 65_535).map_err(|error| {
		request_error(
			request,
			format!("{name} low word does not fit u16: {error}"),
		)
	})?;
	Ok(f32::from(high) * 65_536.0 + f32::from(low))
}

fn prepared_bool(request: &MaterializationRequest<'_>, name: &str) -> OperationResult<bool> {
	match request.parameters.get(name) {
		Some(PreparedParameter::Bool(value)) => Ok(*value),
		Some(value) => Err(operation_error(
			request.descriptor.id,
			OperationErrorKind::PreparedParameterTypeMismatch,
			format!("prepared parameter {name:?} is {value:?}, expected Bool"),
		)),
		None => Err(operation_error(
			request.descriptor.id,
			OperationErrorKind::MissingPreparedParameter,
			format!("prepared parameter {name:?} is required"),
		)),
	}
}

fn adam_moment_parameters(request: &MaterializationRequest<'_>) -> OperationResult<AdamMomentParameters> {
	let beta_one = unit_interval_parameter(request, "beta_one", false)?;
	let beta_two = unit_interval_parameter(request, "beta_two", false)?;
	let epsilon = finite_positive_parameter(request, "epsilon")?;
	let step = prepared_u64(request.descriptor.id, request.parameters, "step")?;
	if step == 0 {
		return Err(request_error(request, "step must be positive"));
	}
	let step =
		i32::try_from(step).map_err(|error| request_error(request, format!("step does not fit int32: {error}")))?;
	let next_step = step
		.checked_add(1)
		.ok_or_else(|| request_error(request, "step + 1 exceeds int32"))?;
	let first_correction = 1.0 - beta_one.powi(step);
	let next_first_correction = 1.0 - beta_one.powi(next_step);
	let second_correction = 1.0 - beta_two.powi(step);
	for (name, value) in [
		("first bias correction", first_correction),
		("next first bias correction", next_first_correction),
		("second bias correction", second_correction),
	] {
		if !(value.is_finite() && value > 0.0) {
			return Err(request_error(
				request,
				format!("{name} must be finite and positive"),
			));
		}
	}
	Ok(AdamMomentParameters {
		beta_one,
		beta_two,
		epsilon,
		first_correction,
		next_first_correction,
		second_correction,
	})
}

fn require_equal_f32_tensors<'a>(
	request: &MaterializationRequest<'_>,
	reference: &Tensor,
	tensors: impl IntoIterator<Item = (&'a str, &'a Tensor)>,
) -> OperationResult<()> {
	require_dtype(request, reference, DType::F32, "weight")?;
	if reference.shape.is_empty() {
		return Err(operation_error(
			request.descriptor.id,
			OperationErrorKind::UnsupportedConcreteShape,
			"optimizer tensors must contain at least one element",
		));
	}
	for (name, tensor) in tensors {
		require_dtype(request, tensor, DType::F32, name)?;
		require_same_tensor_contract(request, reference, tensor, name)?;
	}
	Ok(())
}

fn finite_nonnegative_parameter(request: &MaterializationRequest<'_>, name: &str) -> OperationResult<f32> {
	let value = prepared_f32(request, name)?;
	if value.is_finite() && value >= 0.0 {
		Ok(value)
	} else {
		Err(request_error(
			request,
			format!("{name} must be a finite nonnegative f32"),
		))
	}
}

fn finite_positive_parameter(request: &MaterializationRequest<'_>, name: &str) -> OperationResult<f32> {
	let value = prepared_f32(request, name)?;
	if value.is_finite() && value > 0.0 {
		Ok(value)
	} else {
		Err(request_error(
			request,
			format!("{name} must be a finite positive f32"),
		))
	}
}

fn unit_interval_parameter(
	request: &MaterializationRequest<'_>,
	name: &str,
	include_one: bool,
) -> OperationResult<f32> {
	let value = prepared_f32(request, name)?;
	let upper_valid = match include_one {
		true => value <= 1.0,
		false => value < 1.0,
	};
	if value.is_finite() && value >= 0.0 && upper_valid {
		Ok(value)
	} else {
		let upper = match include_one {
			true => "closed",
			false => "open",
		};
		Err(request_error(
			request,
			format!("{name} must be finite in the {upper} unit interval"),
		))
	}
}

fn all_axes(tensor: &Tensor) -> OperationResult<AxisSet> {
	AxisSet::new((0..tensor.shape.rank()).collect()).map_err(|error| {
		OperationError::new(
			OperationErrorKind::GraphMaterializationFailed,
			error.to_string(),
		)
	})
}

fn sum_reduction(
	request: &MaterializationRequest<'_>,
	axes: AxisSet,
	keep_dimensions: bool,
	tree_lanes: u32,
) -> OperationResult<PrimitiveKind> {
	let rank = request
		.inputs
		.iter()
		.find_map(|tensor| (tensor.name == request.iteration_shape_input).then_some(tensor.tensor.shape.rank()))
		.ok_or_else(|| {
			request_error(
				request,
				"iteration shape input disappeared before reduction construction",
			)
		})?;
	axes.validate_rank(rank)
		.map_err(|error| language_error(request.descriptor.id, error.to_string()))?;
	Ok(PrimitiveKind::Reduce(Reduce {
		operator: ReduceOperator::Sum,
		axes,
		keep_dimensions,
		result: ReduceResult::Value,
		tree_lanes,
	}))
}

fn scalar_f32(
	operation: OperationId,
	builder: &mut ScalarProgramBuilder,
	value: f32,
) -> OperationResult<recipe_language::ScalarExpression> {
	builder
		.f32(value)
		.map_err(|error| language_error(operation, error.to_string()))
}

fn scalar_unary(
	operation: OperationId,
	builder: &mut ScalarProgramBuilder,
	opcode: ScalarOpcode,
	value: recipe_language::ScalarExpression,
) -> OperationResult<recipe_language::ScalarExpression> {
	builder
		.unary(opcode, value)
		.map_err(|error| language_error(operation, error.to_string()))
}

fn scalar_ternary(
	operation: OperationId,
	builder: &mut ScalarProgramBuilder,
	opcode: ScalarOpcode,
	first: recipe_language::ScalarExpression,
	second: recipe_language::ScalarExpression,
	third: recipe_language::ScalarExpression,
) -> OperationResult<recipe_language::ScalarExpression> {
	builder
		.ternary(opcode, first, second, third)
		.map_err(|error| language_error(operation, error.to_string()))
}

fn require_nonnegative(
	operation: OperationId,
	builder: &mut ScalarProgramBuilder,
	value: recipe_language::ScalarExpression,
) -> OperationResult<()> {
	let zero = scalar_f32(operation, builder, 0.0)?;
	let valid = scalar_binary(
		operation,
		builder,
		ScalarOpcode::GreaterThanOrEqual,
		value,
		zero,
	)?;
	scalar_unary(operation, builder, ScalarOpcode::Require, valid)?;
	Ok(())
}

fn require_positive(
	operation: OperationId,
	builder: &mut ScalarProgramBuilder,
	value: recipe_language::ScalarExpression,
) -> OperationResult<()> {
	let zero = scalar_f32(operation, builder, 0.0)?;
	let valid = scalar_binary(operation, builder, ScalarOpcode::GreaterThan, value, zero)?;
	scalar_unary(operation, builder, ScalarOpcode::Require, valid)?;
	Ok(())
}

fn bias_correct(
	operation: OperationId,
	builder: &mut ScalarProgramBuilder,
	value: recipe_language::ScalarExpression,
	correction: f32,
) -> OperationResult<recipe_language::ScalarExpression> {
	let correction = scalar_f32(operation, builder, correction)?;
	scalar_binary(operation, builder, ScalarOpcode::Divide, value, correction)
}

fn require_exact_abi(
	request: &MaterializationRequest<'_>,
	input_names: &[&str],
	output_names: &[&str],
	parameter_names: &[&str],
) -> OperationResult<()> {
	let expected_inputs = input_names.iter().copied().collect::<BTreeSet<_>>();
	let actual_inputs = request
		.inputs
		.iter()
		.map(|tensor| tensor.name)
		.collect::<BTreeSet<_>>();
	let expected_outputs = output_names.iter().copied().collect::<BTreeSet<_>>();
	let actual_outputs = request
		.outputs
		.iter()
		.map(|tensor| tensor.name)
		.collect::<BTreeSet<_>>();
	let expected_parameters = parameter_names.iter().copied().collect::<BTreeSet<_>>();
	let actual_parameters = request
		.parameters
		.keys()
		.map(String::as_str)
		.collect::<BTreeSet<_>>();
	if actual_inputs != expected_inputs {
		return Err(request_error(
			request,
			format!("input tensor ABI mismatch: expected {expected_inputs:?}, got {actual_inputs:?}"),
		));
	}
	if actual_outputs != expected_outputs {
		return Err(request_error(
			request,
			format!("output tensor ABI mismatch: expected {expected_outputs:?}, got {actual_outputs:?}"),
		));
	}
	if actual_parameters != expected_parameters {
		return Err(request_error(
			request,
			format!(
				"prepared parameter ABI mismatch: expected {expected_parameters:?}, got {actual_parameters:?}"
			),
		));
	}
	Ok(())
}

fn validate_request(request: &MaterializationRequest<'_>) -> OperationResult<()> {
	if !matches!(
		request.descriptor.lowering,
		LoweringAvailability::Composition(_)
	) {
		return Err(operation_error(
			request.descriptor.id,
			OperationErrorKind::WrongLoweringKind,
			"materialization requires a structured operation descriptor",
		));
	}
	if request.inputs.is_empty() || request.outputs.is_empty() {
		return Err(request_error(
			request,
			"materialization requires at least one input and one output declaration",
		));
	}
	let mut names = BTreeSet::new();
	let mut values = BTreeSet::new();
	for declaration in request.inputs.iter().chain(request.outputs) {
		if declaration.name.is_empty() || !names.insert(declaration.name) {
			return Err(request_error(
				request,
				format!(
					"tensor declaration name {:?} is empty or duplicated",
					declaration.name
				),
			));
		}
		if !values.insert(declaration.tensor.id) {
			return Err(request_error(
				request,
				format!(
					"tensor {} is declared more than once",
					declaration.tensor.id
				),
			));
		}
		declaration
			.tensor
			.validate()
			.map_err(|error| language_error(request.descriptor.id, error.to_string()))?;
	}
	for input in request.inputs {
		if !input.tensor.external_input {
			return Err(request_error(
				request,
				format!("input {:?} is not marked external_input", input.name),
			));
		}
	}
	for output in request.outputs {
		if output.tensor.external_input || !output.tensor.external_output {
			return Err(request_error(
				request,
				format!(
					"output {:?} must be non-input and marked external_output",
					output.name
				),
			));
		}
	}
	Ok(())
}

fn input<'a>(request: &'a MaterializationRequest<'a>, name: &str) -> OperationResult<&'a Tensor> {
	request
		.inputs
		.iter()
		.find_map(|declaration| (declaration.name == name).then_some(declaration.tensor))
		.ok_or_else(|| {
			request_error(
				request,
				format!("missing required input tensor declaration {name:?}"),
			)
		})
}

fn output<'a>(request: &'a MaterializationRequest<'a>, name: &str) -> OperationResult<&'a Tensor> {
	request
		.outputs
		.iter()
		.find_map(|declaration| (declaration.name == name).then_some(declaration.tensor))
		.ok_or_else(|| {
			request_error(
				request,
				format!("missing required output tensor declaration {name:?}"),
			)
		})
}

fn require_dtype(
	request: &MaterializationRequest<'_>,
	tensor: &Tensor,
	expected: DType,
	name: &str,
) -> OperationResult<()> {
	if tensor.dtype == expected {
		Ok(())
	} else {
		Err(request_error(
			request,
			format!("{name} is {:?}, expected {expected:?}", tensor.dtype),
		))
	}
}

fn require_shape(
	request: &MaterializationRequest<'_>,
	tensor: &Tensor,
	expected: &[u64],
	name: &str,
) -> OperationResult<()> {
	if tensor.shape.extents() == expected {
		Ok(())
	} else {
		Err(operation_error(
			request.descriptor.id,
			OperationErrorKind::UnsupportedConcreteShape,
			format!(
				"{name} has shape {:?}, concrete {} materializer requires {expected:?}",
				tensor.shape.extents(),
				request.descriptor.symbol
			),
		))
	}
}

fn require_same_tensor_contract(
	request: &MaterializationRequest<'_>,
	expected: &Tensor,
	actual: &Tensor,
	name: &str,
) -> OperationResult<()> {
	if actual.dtype == expected.dtype && actual.shape == expected.shape {
		Ok(())
	} else {
		Err(request_error(
			request,
			format!(
				"{name} has ({:?}, {:?}), expected ({:?}, {:?})",
				actual.dtype,
				actual.shape.extents(),
				expected.dtype,
				expected.shape.extents()
			),
		))
	}
}

fn prepared_u64(operation: OperationId, parameters: &PreparedParameters, name: &str) -> OperationResult<u64> {
	match parameters.get(name) {
		Some(PreparedParameter::U64(value)) => Ok(*value),
		Some(value) => Err(operation_error(
			operation,
			OperationErrorKind::PreparedParameterTypeMismatch,
			format!("prepared parameter {name:?} is {value:?}, expected U64"),
		)),
		None => Err(operation_error(
			operation,
			OperationErrorKind::MissingPreparedParameter,
			format!("prepared parameter {name:?} is required"),
		)),
	}
}

fn prepared_f32(request: &MaterializationRequest<'_>, name: &str) -> OperationResult<f32> {
	match request.parameters.get(name) {
		Some(PreparedParameter::F32Bits(bits)) => Ok(f32::from_bits(*bits)),
		Some(value) => Err(operation_error(
			request.descriptor.id,
			OperationErrorKind::PreparedParameterTypeMismatch,
			format!("prepared parameter {name:?} is {value:?}, expected F32Bits"),
		)),
		None => Err(operation_error(
			request.descriptor.id,
			OperationErrorKind::MissingPreparedParameter,
			format!("prepared parameter {name:?} is required"),
		)),
	}
}

fn require_true(request: &MaterializationRequest<'_>, name: &str) -> OperationResult<()> {
	match request.parameters.get(name) {
		Some(PreparedParameter::Bool(true)) => Ok(()),
		Some(PreparedParameter::Bool(false)) => Err(request_error(
			request,
			format!("prepared fact {name:?} must be true"),
		)),
		Some(value) => Err(operation_error(
			request.descriptor.id,
			OperationErrorKind::PreparedParameterTypeMismatch,
			format!("prepared parameter {name:?} is {value:?}, expected Bool"),
		)),
		None => Err(operation_error(
			request.descriptor.id,
			OperationErrorKind::MissingPreparedParameter,
			format!("prepared fact {name:?} is required"),
		)),
	}
}

fn prepared_axis(request: &MaterializationRequest<'_>, name: &str) -> OperationResult<usize> {
	usize::try_from(prepared_u64(
		request.descriptor.id,
		request.parameters,
		name,
	)?)
	.map_err(|error| {
		request_error(
			request,
			format!("prepared axis {name:?} does not fit usize: {error}"),
		)
	})
}

fn prepared_tree_lanes(request: &MaterializationRequest<'_>) -> OperationResult<u32> {
	let value = prepared_u64(request.descriptor.id, request.parameters, "tree_lanes")?;
	let lanes = u32::try_from(value)
		.map_err(|error| request_error(request, format!("tree_lanes does not fit u32: {error}")))?;
	if lanes == 0 || lanes > 1024 || !lanes.is_power_of_two() {
		return Err(request_error(
			request,
			"tree_lanes must be a power of two in 1..=1024",
		));
	}
	Ok(lanes)
}

fn scalar_builder(operation: OperationId) -> OperationResult<ScalarProgramBuilder> {
	ScalarProgramBuilder::new().map_err(|error| language_error(operation, error.to_string()))
}

fn scalar_input(
	operation: OperationId,
	builder: &mut ScalarProgramBuilder,
	dtype: DType,
) -> OperationResult<recipe_language::ScalarExpression> {
	builder
		.input(dtype)
		.map_err(|error| language_error(operation, error.to_string()))
}

fn scalar_binary(
	operation: OperationId,
	builder: &mut ScalarProgramBuilder,
	opcode: ScalarOpcode,
	left: recipe_language::ScalarExpression,
	right: recipe_language::ScalarExpression,
) -> OperationResult<recipe_language::ScalarExpression> {
	builder
		.binary(opcode, left, right)
		.map_err(|error| language_error(operation, error.to_string()))
}

fn scalar_finish(
	operation: OperationId,
	builder: ScalarProgramBuilder,
	outputs: &[recipe_language::ScalarExpression],
) -> OperationResult<recipe_core::ScalarProgram> {
	builder
		.finish(outputs)
		.map_err(|error| language_error(operation, error.to_string()))
}

fn forbidden_aliases(inputs: usize, outputs: usize) -> Vec<PrimitiveAliasRule> {
	(0..inputs)
		.flat_map(|input| {
			(0..outputs).map(move |output| PrimitiveAliasRule {
				input,
				output,
				permission: AliasPermission::Forbidden,
			})
		})
		.collect()
}

const fn primitive_family(kind: &PrimitiveKind) -> PrimitiveFamily {
	match kind {
		PrimitiveKind::Elementwise(_) => PrimitiveFamily::Elementwise,
		PrimitiveKind::Reduce(_) => PrimitiveFamily::Reduce,
		PrimitiveKind::Scan(_) => PrimitiveFamily::Scan,
		PrimitiveKind::Contraction(_) => PrimitiveFamily::Contraction,
		PrimitiveKind::Gather(_) => PrimitiveFamily::Gather,
		PrimitiveKind::Scatter(_) => PrimitiveFamily::Scatter,
		PrimitiveKind::Histogram(_) => PrimitiveFamily::Histogram,
		PrimitiveKind::Sort(_) => PrimitiveFamily::Sort,
		PrimitiveKind::Random(_) => PrimitiveFamily::Random,
	}
}

fn request_error(request: &MaterializationRequest<'_>, detail: impl Into<String>) -> OperationError {
	operation_error(
		request.descriptor.id,
		OperationErrorKind::InvalidMaterializationRequest,
		detail,
	)
}

fn language_error(operation: OperationId, detail: impl Into<String>) -> OperationError {
	operation_error(
		operation,
		OperationErrorKind::GraphMaterializationFailed,
		detail,
	)
}

fn operation_error(operation: OperationId, kind: OperationErrorKind, detail: impl Into<String>) -> OperationError {
	OperationError::new(kind, detail).for_operation(operation)
}

fn ceiling_log_two(value: u64) -> u64 {
	match value {
		0 | 1 => 0,
		extent => u64::from(u64::BITS - (extent - 1).leading_zeros()),
	}
}

fn identity_ranges(namespace: IdentityNamespace) -> OperationResult<(u64, u64, u64, u64)> {
	let value_start = namespace.first_value.get();
	let value_end = value_start
		.checked_add(namespace.value_capacity)
		.ok_or_else(|| {
			OperationError::new(
				OperationErrorKind::IdentityNamespaceExhausted,
				"reserved intermediate value identity range exceeds u64",
			)
		})?;
	let kernel_start = namespace.first_kernel.get();
	let kernel_end = kernel_start
		.checked_add(namespace.kernel_capacity)
		.ok_or_else(|| {
			OperationError::new(
				OperationErrorKind::IdentityNamespaceExhausted,
				"reserved kernel identity range exceeds u64",
			)
		})?;
	Ok((value_start, value_end, kernel_start, kernel_end))
}

const fn ranges_overlap(left_start: u64, left_end: u64, right_start: u64, right_end: u64) -> bool {
	left_start < left_end && right_start < right_end && left_start < right_end && right_start < left_end
}

fn has_concrete_materializer(descriptor: OperationDescriptor) -> bool {
	optimizer_normalization::supports(descriptor)
		|| solver_fft::supports(descriptor)
		|| attention_sequence_embedding::supports(descriptor)
		|| convolution_pooling::supports(descriptor)
		|| loss_metrics::supports(descriptor)
		|| indexing_sort_encoding::supports(descriptor)
		|| graph_cluster_rl::supports(descriptor)
		|| tree_boosting::supports(descriptor)
		|| inference_quantization_diffusion::supports(descriptor)
		|| creation_shape_misc::supports(descriptor)
}
