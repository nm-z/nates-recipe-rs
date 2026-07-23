use std::collections::{BTreeMap, BTreeSet};

use recipe_core::{
	AliasPermission, AliasRule, ArenaLayout, ArenaObject, ArenaObjectId, ArenaRelease, ArtifactBuildAccess,
	ArtifactBuildBinding, ArtifactBuildProvenance, ArtifactBuildRecipe, ArtifactBuildView, ArtifactDispatchGeometry,
	ArtifactId, ArtifactIdentity, ArtifactWorkBounds, ByteCount, ByteOffset, CalculationTask, CandidateIdentity,
	CapacityLedger, CompletionSlot, CompletionSlotId, DType, DeviceBytes, DeviceId, DeviceKind, Digest,
	DiscoveryProfile, DraftIdentity, DraftPlan, InitDataImage, InitDataImageMember, KernelResourceBounds,
	KernelTemplate, KernelTemplateId, LinkId, MetricId, MetricPurpose, MetricSlot, MetricSlotId, MetricTask,
	Nanoseconds, QueueSlot, QueueSlotId, ReservationLedger, ResourceManifest, RunPhase, ScalarLiteral,
	ScheduleWindow, StaticBufferAccess, SubmissionSlots, Task, TaskId, TaskKind, Topology, TransferEndpoint,
	TransferLaneClaim, TransferTask, ValueAliasContract, ValueBinding, ValueId, ValueSpec,
};
use recipe_language::{CalculationGraph, Tensor};
use recipe_primitives::{
	AccessMode, BufferLifetime, BufferOrigin, LoweredProgram, ProgramBuffer, ProgramStage, StageKind,
};
use recipe_scheduler::{ScheduleErrorKind, UnscheduledTask, pack_arenas, schedule};

use crate::error::{PlannerError, PlannerErrorKind, PlannerResult};
use crate::hash::StableDigest;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct KernelPlacement {
	pub kernel: KernelTemplateId,
	pub device: DeviceId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StagePlacement {
	pub source_kernel: KernelTemplateId,
	pub stage_ordinal: u32,
	pub kernel_template: KernelTemplateId,
	pub device: DeviceId,
	pub artifact: ArtifactId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct LogicalValueCopy {
	pub logical: ValueId,
	pub device: DeviceId,
	pub physical: ValueId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannedCandidate {
	pub draft: DraftPlan,
	pub arena_layouts: Vec<ArenaLayout>,
	pub makespan: Nanoseconds,
	pub placements: Vec<KernelPlacement>,
	pub stage_placements: Vec<StagePlacement>,
	pub lowered_programs: Vec<LoweredProgram>,
	pub value_copies: Vec<LogicalValueCopy>,
}

/// Ranked, one-shot candidate stream. Once returned, an identity is never
/// returned again, whether or not the caller explicitly rejects it.
#[derive(Clone, Debug)]
pub struct PlannerSearch {
	ranked: Vec<PlannedCandidate>,
	cursor: usize,
	issued: BTreeSet<CandidateIdentity>,
	rejected: BTreeSet<CandidateIdentity>,
}

impl PlannerSearch {
	#[must_use]
	pub fn ranked_candidates(&self) -> &[PlannedCandidate] {
		&self.ranked
	}

	pub fn next_candidate(&mut self) -> Option<&PlannedCandidate> {
		while self.cursor < self.ranked.len() {
			let index = self.cursor;
			self.cursor += 1;
			let identity = self.ranked[index].draft.candidate;
			if self.rejected.contains(&identity) {
				continue;
			}
			self.issued.insert(identity);
			return self.ranked.get(index);
		}
		None
	}

	pub fn reject(&mut self, identity: CandidateIdentity) -> PlannerResult<()> {
		if !self.issued.contains(&identity) {
			return Err(PlannerError::new(
				PlannerErrorKind::UnknownCandidate,
				"only a previously issued candidate can be rejected",
			));
		}
		if !self.rejected.insert(identity) {
			return Err(PlannerError::new(
				PlannerErrorKind::AlreadyRejected,
				"candidate was already rejected",
			));
		}
		Ok(())
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct PlacementOption {
	device: DeviceId,
}

#[derive(Clone, Debug)]
struct Assignment {
	options: Vec<PlacementOption>,
}

#[derive(Clone, Debug)]
struct LoweredKernelPlan {
	program: LoweredProgram,
	stage_templates: Vec<KernelTemplateId>,
}

/// Exhaustively enumerates the finite placement/artifact product, lowers every
/// feasible assignment, and ranks valid candidates by measured makespan and
/// then stable candidate identity.
pub fn plan_candidates(
	graph: &CalculationGraph,
	topology: &Topology,
	discovery: &DiscoveryProfile,
	artifacts: &[ArtifactIdentity],
	reservations: &ReservationLedger,
	capacity: &CapacityLedger,
) -> PlannerResult<PlannerSearch> {
	graph.validate()
		.map_err(|error| PlannerError::new(PlannerErrorKind::InvalidGraph, error.to_string()))?;
	topology
		.validate()
		.map_err(|errors| PlannerError::new(PlannerErrorKind::InvalidTopology, errors.to_string()))?;
	topology
		.validate_scheduling_properties()
		.map_err(|errors| PlannerError::new(PlannerErrorKind::InvalidTopology, errors.to_string()))?;
	discovery
		.validate(topology)
		.map_err(|errors| PlannerError::new(PlannerErrorKind::InvalidDiscovery, errors.to_string()))?;
	reservations
		.validate(topology)
		.map_err(|errors| PlannerError::new(PlannerErrorKind::InvalidReservation, errors.to_string()))?;
	capacity
		.validate(topology, reservations)
		.map_err(|errors| PlannerError::new(PlannerErrorKind::InvalidCapacity, errors.to_string()))?;

	let order = graph
		.topological_order()
		.map_err(|error| PlannerError::new(PlannerErrorKind::InvalidGraph, error.to_string()))?;
	let tensors = tensor_index(graph);
	let nodes = graph
		.nodes
		.iter()
		.map(|node| (node.kernel.id, node))
		.collect::<BTreeMap<_, _>>();
	let (programs, templates) = lower_programs(graph, &tensors)?;
	validate_artifact_catalog(artifacts)?;
	let choices = legal_choices(&order, topology, discovery)?;
	let graph_identity = graph_digest(&order, &nodes, &tensors, &programs)?;
	let context = PlanningContext {
		order: &order,
		nodes: &nodes,
		tensors: &tensors,
		programs: &programs,
		templates: &templates,
		topology,
		discovery,
		artifacts,
		capacity,
	};

	let mut ranked = Vec::new();
	let mut identities = BTreeSet::new();
	let mut candidate_failures = Vec::new();
	enumerate_assignments(&choices, &mut Vec::new(), &mut |options| {
		let assignment = Assignment {
			options: options.to_vec(),
		};
		let candidate = candidate_identity(graph_identity, topology, discovery, &order, &assignment);
		if !identities.insert(candidate) {
			return Err(PlannerError::new(
				PlannerErrorKind::IdentityCollision,
				"two distinct placement assignments produced one candidate identity",
			));
		}
		match lower_candidate(&context, &assignment, candidate) {
			Ok(planned) => ranked.push(planned),
			Err(error)
				if matches!(
					error.kind,
					PlannerErrorKind::NoRoute
						| PlannerErrorKind::DependencyConflict
						| PlannerErrorKind::CandidateInfeasible
						| PlannerErrorKind::InvalidCapacity
				) =>
			{
				candidate_failures.push(error.to_string());
			}
			Err(error) => return Err(error),
		}
		Ok(())
	})?;

	if ranked.is_empty() {
		let detail = candidate_failures
			.first()
			.map(String::as_str)
			.unwrap_or("no assignment could be lowered");
		return Err(PlannerError::new(
			PlannerErrorKind::NoViableCandidate,
			format!(
				"all {} finite placement assignments failed; first failure: {detail}",
				identities.len()
			),
		));
	}
	ranked.sort_by_key(|candidate| (candidate.makespan, candidate.draft.candidate));
	Ok(PlannerSearch {
		ranked,
		cursor: 0,
		issued: BTreeSet::new(),
		rejected: BTreeSet::new(),
	})
}

fn tensor_index(graph: &CalculationGraph) -> BTreeMap<ValueId, &Tensor> {
	graph.tensors
		.iter()
		.map(|tensor| (tensor.id, tensor))
		.collect()
}

fn lower_programs(
	graph: &CalculationGraph,
	tensors: &BTreeMap<ValueId, &Tensor>,
) -> PlannerResult<(
	BTreeMap<KernelTemplateId, LoweredKernelPlan>,
	BTreeMap<KernelTemplateId, KernelTemplate>,
)> {
	let mut programs = BTreeMap::new();
	let mut templates = BTreeMap::new();
	let mut stage_identities = BTreeMap::<KernelTemplateId, (KernelTemplateId, u32)>::new();
	for node in &graph.nodes {
		let program = recipe_primitives::lower(&node.kernel, tensors).map_err(|error| {
			PlannerError::new(
				PlannerErrorKind::InvalidGraph,
				format!(
					"primitive lowering failed for kernel {}: {error}",
					node.kernel.id
				),
			)
		})?;
		program.validate().map_err(|errors| {
			PlannerError::new(
				PlannerErrorKind::InvalidGraph,
				format!(
					"primitive lowering for kernel {} produced an invalid program: {}",
					node.kernel.id,
					errors.iter()
						.map(ToString::to_string)
						.collect::<Vec<_>>()
						.join("; ")
				),
			)
		})?;
		let mut stage_templates = Vec::with_capacity(program.stages.len());
		for stage in &program.stages {
			let identity = stage_template_identity(&program, stage)?;
			match stage_identities.insert(identity, (program.source_kernel, stage.id.get())) {
				Some(previous) if previous != (program.source_kernel, stage.id.get()) => {
					return Err(PlannerError::new(
						PlannerErrorKind::IdentityCollision,
						format!(
							"stage identity {identity} collides between ({}, {}) and ({}, {})",
							previous.0,
							previous.1,
							program.source_kernel,
							stage.id.get()
						),
					));
				}
				Some(_) | None => {}
			}
			stage_templates.push(identity);
			let StageKind::ScalarMap { template } = &stage.kind else {
				continue;
			};
			let mut template = template.clone();
			template.id = identity;
			template
				.validate()
				.map_err(|errors| PlannerError::new(PlannerErrorKind::InvalidGraph, errors.to_string()))?;
			if templates.insert(identity, template).is_some() {
				return Err(PlannerError::new(
					PlannerErrorKind::IdentityCollision,
					format!("scalar stage template {identity} appears more than once"),
				));
			}
		}
		programs.insert(
			program.source_kernel,
			LoweredKernelPlan {
				program,
				stage_templates,
			},
		);
	}
	Ok((programs, templates))
}

fn stage_template_identity(program: &LoweredProgram, stage: &ProgramStage) -> PlannerResult<KernelTemplateId> {
	let mut digest = StableDigest::new("recipe-planner-stage-template-v1");
	digest.bytes(&program.digest.bytes());
	digest.u64(program.source_kernel.get());
	digest.u64(u64::from(stage.id.get()));
	let bytes = digest.finish().bytes();
	let prefix: [u8; 8] = bytes[..8].try_into().map_err(|error| {
		PlannerError::new(
			PlannerErrorKind::InvalidDraft,
			format!("stage identity digest width changed: {error}"),
		)
	})?;
	let identity = KernelTemplateId::new(u64::from_le_bytes(prefix));
	if identity.get() == 0 {
		return Err(PlannerError::new(
			PlannerErrorKind::IdentityCollision,
			format!(
				"stage ({}, {}) produced the reserved zero identity",
				program.source_kernel,
				stage.id.get()
			),
		));
	}
	Ok(identity)
}

fn validate_artifact_catalog(artifacts: &[ArtifactIdentity]) -> PlannerResult<()> {
	let mut ids = BTreeSet::new();
	for artifact in artifacts {
		if artifact.id.get() == 0 || !ids.insert(artifact.id) {
			return Err(PlannerError::new(
				PlannerErrorKind::InvalidArtifact,
				format!("artifact ID {} is zero or duplicated", artifact.id),
			));
		}
		artifact.validate().map_err(|errors| {
			PlannerError::new(
				PlannerErrorKind::InvalidArtifact,
				format!("artifact {}: {errors}", artifact.id),
			)
		})?;
	}
	Ok(())
}

fn legal_choices(
	order: &[KernelTemplateId],
	topology: &Topology,
	discovery: &DiscoveryProfile,
) -> PlannerResult<Vec<Vec<PlacementOption>>> {
	if order.is_empty() {
		return Ok(Vec::new());
	}
	let gpus = topology
		.devices
		.iter()
		.filter(|device| device.kind == DeviceKind::GpuMemory)
		.filter(|device| {
			discovery
				.device(device.id)
				.is_some_and(|entry| entry.calculation.is_some())
		})
		.map(|device| device.id)
		.collect::<Vec<_>>();
	if gpus.is_empty() {
		return Err(PlannerError::new(
			PlannerErrorKind::NoCalculationDevice,
			"no measured GPU calculation capability is available",
		));
	}
	let mut result = Vec::with_capacity(order.len());
	for _kernel in order {
		let mut options = gpus
			.iter()
			.copied()
			.map(|device| PlacementOption { device })
			.collect::<Vec<_>>();
		options.sort();
		result.push(options);
	}
	Ok(result)
}

fn enumerate_assignments(
	choices: &[Vec<PlacementOption>],
	current: &mut Vec<PlacementOption>,
	visit: &mut impl FnMut(&[PlacementOption]) -> PlannerResult<()>,
) -> PlannerResult<()> {
	if current.len() == choices.len() {
		return visit(current);
	}
	let index = current.len();
	for option in &choices[index] {
		current.push(*option);
		enumerate_assignments(choices, current, visit)?;
		current.pop();
	}
	Ok(())
}

fn graph_digest(
	order: &[KernelTemplateId],
	nodes: &BTreeMap<KernelTemplateId, &recipe_language::CalculationNode>,
	tensors: &BTreeMap<ValueId, &Tensor>,
	programs: &BTreeMap<KernelTemplateId, LoweredKernelPlan>,
) -> PlannerResult<recipe_core::Digest> {
	let mut digest = StableDigest::new("recipe-planner-graph-v4");
	digest.length(tensors.len());
	for tensor in tensors.values() {
		digest.u64(tensor.id.get());
		digest.u8(dtype_tag(tensor.dtype));
		digest.u64(tensor.storage_bytes.get());
		digest.bool(tensor.external_input);
		digest.bool(tensor.external_output);
		digest.u64(tensor.layout.offset_elements);
		digest.length(tensor.shape.extents().len());
		for extent in tensor.shape.extents() {
			digest.u64(*extent);
		}
		digest.length(tensor.layout.strides.len());
		for stride in &tensor.layout.strides {
			digest.u64(*stride);
		}
	}
	digest.length(order.len());
	for kernel in order {
		let node = nodes.get(kernel).ok_or_else(|| {
			PlannerError::new(
				PlannerErrorKind::InvalidGraph,
				format!("kernel {kernel} disappeared during graph hashing"),
			)
		})?;
		digest.u64(kernel.get());
		digest.length(node.kernel.inputs.len());
		for input in &node.kernel.inputs {
			digest.u64(input.get());
		}
		digest.length(node.kernel.outputs.len());
		for output in &node.kernel.outputs {
			digest.u64(output.get());
		}
		digest.u64(node
			.kernel
			.work(tensors)
			.map_err(|error| PlannerError::new(PlannerErrorKind::InvalidGraph, error.to_string()))?
			.get());
		let lowered = programs.get(kernel).ok_or_else(|| {
			PlannerError::new(
				PlannerErrorKind::InvalidGraph,
				format!("kernel {kernel} has no lowered program during graph hashing"),
			)
		})?;
		digest.bytes(&lowered.program.digest.bytes());
		digest.length(lowered.stage_templates.len());
		for stage_template in &lowered.stage_templates {
			digest.u64(stage_template.get());
		}
	}
	Ok(digest.finish())
}

fn hash_kernel_template(digest: &mut StableDigest, template: &KernelTemplate) {
	digest.length(template.index_space.dimensions().len());
	for dimension in template.index_space.dimensions() {
		digest.u64(dimension.get());
	}
	digest.length(template.inputs.len());
	for input in &template.inputs {
		digest.u64(input.id.get());
		digest.u8(dtype_tag(input.dtype));
		hash_static_access(digest, &input.access);
	}
	digest.length(template.outputs.len());
	for output in &template.outputs {
		digest.u64(output.id.get());
		digest.u8(dtype_tag(output.dtype));
		hash_static_access(digest, &output.access);
	}
	digest.length(template.program.inputs.len());
	for input in &template.program.inputs {
		digest.u64(input.id.get());
		digest.u8(dtype_tag(input.dtype));
	}
	digest.length(template.program.constants.len());
	for constant in &template.program.constants {
		digest.u64(constant.id.get());
		match constant.value {
			ScalarLiteral::F32Bits(bits) => {
				digest.u8(0);
				digest.u64(u64::from(bits));
			}
			ScalarLiteral::I32(value) => {
				digest.u8(1);
				digest.bytes(&value.to_le_bytes());
			}
		}
	}
	digest.length(template.program.instructions.len());
	for instruction in &template.program.instructions {
		digest.u64(instruction.result.get());
		digest.u8(dtype_tag(instruction.dtype));
		digest.bytes(format!("{:?}", instruction.opcode).as_bytes());
		digest.length(instruction.operands.len());
		for operand in &instruction.operands {
			digest.u64(operand.get());
		}
	}
	digest.length(template.program.outputs.len());
	for output in &template.program.outputs {
		digest.u64(output.get());
	}
	digest.length(template.alias_rules.len());
	for rule in &template.alias_rules {
		digest.u64(rule.input.get());
		digest.u64(rule.output.get());
		digest.u8(alias_permission_tag(rule.permission));
	}
}

fn hash_static_access(digest: &mut StableDigest, access: &StaticBufferAccess) {
	digest.u64(access.offset_elements);
	digest.length(access.strides.len());
	for stride in &access.strides {
		digest.u64(*stride);
	}
	digest.u64(access.storage_bytes.get());
}

const fn alias_permission_tag(permission: AliasPermission) -> u8 {
	match permission {
		AliasPermission::Forbidden => 0,
		AliasPermission::MayAliasExact => 1,
		AliasPermission::MustAliasExact => 2,
	}
}

const fn dtype_tag(dtype: DType) -> u8 {
	match dtype {
		DType::F32 => 0,
		DType::I32 => 1,
	}
}

fn candidate_identity(
	graph: recipe_core::Digest,
	topology: &Topology,
	discovery: &DiscoveryProfile,
	order: &[KernelTemplateId],
	assignment: &Assignment,
) -> CandidateIdentity {
	let mut digest = StableDigest::new("recipe-planner-candidate-v3");
	digest.digest(graph);
	digest.digest(topology.identity.digest());
	digest.digest(discovery.identity.digest());
	digest.length(order.len());
	for (kernel, option) in order.iter().zip(&assignment.options) {
		digest.u64(kernel.get());
		digest.u64(option.device.get());
	}
	CandidateIdentity::new(digest.finish())
}

fn hash_artifact(digest: &mut StableDigest, artifact: &ArtifactIdentity) {
	digest.u64(artifact.id.get());
	digest.digest(artifact.digest);
	digest.label(&artifact.format);
	digest.label(&artifact.target.backend);
	digest.label(&artifact.target.architecture);
	digest.label(&artifact.target.abi);
	digest.label(&artifact.toolchain.name);
	digest.label(&artifact.toolchain.version);
	digest.digest(artifact.toolchain.digest);
	digest.label(&artifact.entry_symbol);
	digest.u64(artifact.kernel_template.get());
	digest.u64(artifact.resources.private_bytes_per_lane.get());
	digest.u64(artifact.resources.shared_bytes_per_workgroup.get());
	digest.u64(artifact.resources.scratch_bytes_per_dispatch.get());
	digest.u64(u64::from(artifact.resources.maximum_workgroup_lanes));
	match artifact.build {
		Some(build) => {
			digest.u8(1);
			digest.digest(build.program_digest);
			digest.u64(u64::from(build.stage_ordinal));
			digest.digest(build.contract_digest);
		}
		None => digest.u8(0),
	}
}

#[derive(Clone, Copy, Debug)]
struct RuntimeCopy {
	device: DeviceId,
	value: ValueId,
	producer: TaskId,
}

#[derive(Debug)]
struct TransferChain {
	final_value: ValueId,
	final_task: TaskId,
	values: Vec<ValueSpec>,
	tasks: Vec<UnscheduledTask>,
	submission_devices: Vec<DeviceId>,
}

#[derive(Clone, Copy, Debug)]
struct TransferAllocationStart {
	task: u64,
	value: u64,
}

#[derive(Clone, Debug)]
struct ImageRegion {
	device: DeviceId,
	image: ValueId,
	bytes: ByteCount,
	members: Vec<(ValueId, ByteOffset)>,
	data_members: Vec<InitDataImageMember>,
}

#[derive(Clone, Debug)]
struct AliasInvocation {
	task: TaskId,
	inputs: Vec<ValueId>,
	outputs: Vec<ValueId>,
	rules: Vec<AliasRule>,
}

#[derive(Debug)]
struct LoweredArtifacts<'a> {
	chosen: &'a mut Vec<ArtifactIdentity>,
	builds: &'a mut Vec<ArtifactBuildRecipe>,
	placements: &'a mut Vec<StagePlacement>,
}

#[derive(Debug)]
struct StableIdAllocator {
	next: u64,
	kind: &'static str,
}

impl StableIdAllocator {
	const fn new(kind: &'static str) -> Self {
		Self { next: 1, kind }
	}

	const fn peek(&self) -> u64 {
		self.next
	}

	fn take(&mut self) -> PlannerResult<u64> {
		let result = self.next;
		self.next = self.next.checked_add(1).ok_or_else(|| {
			PlannerError::new(
				PlannerErrorKind::ArithmeticOverflow,
				format!("{} ID space overflowed", self.kind),
			)
		})?;
		Ok(result)
	}
}

#[derive(Debug)]
struct LoweringState {
	task_ids: StableIdAllocator,
	value_ids: StableIdAllocator,
	values: Vec<ValueSpec>,
	copies: BTreeMap<ValueId, BTreeMap<DeviceId, RuntimeCopy>>,
	logical_copies: Vec<LogicalValueCopy>,
	images: Vec<ImageRegion>,
	tasks: Vec<UnscheduledTask>,
	resources: ResourceManifest,
	aliases: Vec<AliasInvocation>,
	fault_flags: BTreeMap<(KernelTemplateId, u32), ValueId>,
	fault_readbacks: BTreeMap<ValueId, TaskId>,
	init_tasks: BTreeMap<DeviceId, TaskId>,
}

impl LoweringState {
	fn new() -> Self {
		Self {
			task_ids: StableIdAllocator::new("task"),
			value_ids: StableIdAllocator::new("value"),
			values: Vec::new(),
			copies: BTreeMap::new(),
			logical_copies: Vec::new(),
			images: Vec::new(),
			tasks: Vec::new(),
			resources: ResourceManifest {
				queues: Vec::new(),
				completions: Vec::new(),
				metrics: Vec::new(),
				pinned_staging: Vec::new(),
				scratch: Vec::new(),
			},
			aliases: Vec::new(),
			fault_flags: BTreeMap::new(),
			fault_readbacks: BTreeMap::new(),
			init_tasks: BTreeMap::new(),
		}
	}

	fn next_value(&mut self) -> PlannerResult<ValueId> {
		Ok(ValueId::new(self.value_ids.take()?))
	}

	fn next_task(&mut self) -> PlannerResult<TaskId> {
		Ok(TaskId::new(self.task_ids.take()?))
	}

	fn next_submission(&mut self, device: DeviceId) -> PlannerResult<(TaskId, SubmissionSlots)> {
		let task = self.next_task()?;
		let queue = QueueSlotId::new(task.get());
		let completion = CompletionSlotId::new(task.get());
		self.resources.queues.push(QueueSlot { id: queue, device });
		self.resources.completions.push(CompletionSlot {
			id: completion,
			device,
		});
		Ok((task, SubmissionSlots { queue, completion }))
	}

	fn push_copy(&mut self, logical: ValueId, copy: RuntimeCopy) -> PlannerResult<()> {
		let previous = self
			.copies
			.entry(logical)
			.or_default()
			.insert(copy.device, copy);
		if previous.is_some() {
			return Err(PlannerError::new(
				PlannerErrorKind::InvalidDraft,
				format!(
					"logical value {logical} has two copies on device {}",
					copy.device
				),
			));
		}
		self.logical_copies.push(LogicalValueCopy {
			logical,
			device: copy.device,
			physical: copy.value,
		});
		Ok(())
	}
}

#[derive(Debug)]
struct PlanningContext<'a> {
	order: &'a [KernelTemplateId],
	nodes: &'a BTreeMap<KernelTemplateId, &'a recipe_language::CalculationNode>,
	tensors: &'a BTreeMap<ValueId, &'a Tensor>,
	programs: &'a BTreeMap<KernelTemplateId, LoweredKernelPlan>,
	templates: &'a BTreeMap<KernelTemplateId, KernelTemplate>,
	topology: &'a Topology,
	discovery: &'a DiscoveryProfile,
	artifacts: &'a [ArtifactIdentity],
	capacity: &'a CapacityLedger,
}

fn lower_candidate(
	context: &PlanningContext<'_>,
	assignment: &Assignment,
	candidate: CandidateIdentity,
) -> PlannerResult<PlannedCandidate> {
	let selected = context
		.order
		.iter()
		.copied()
		.zip(&assignment.options)
		.collect::<BTreeMap<_, _>>();
	let mut state = LoweringState::new();
	initialize_data_images(
		context.order,
		context.nodes,
		context.tensors,
		context.programs,
		context.topology,
		&selected,
		&mut state,
	)?;

	let mut placements = Vec::with_capacity(context.order.len());
	let mut stage_placements = Vec::new();
	let mut chosen_artifacts = Vec::new();
	let mut artifact_builds = Vec::new();
	for kernel in context.order {
		let node = context.nodes.get(kernel).ok_or_else(|| {
			PlannerError::new(
				PlannerErrorKind::InvalidGraph,
				format!("kernel {kernel} is absent during lowering"),
			)
		})?;
		let option = selected[kernel];
		let lowered = context.programs.get(kernel).ok_or_else(|| {
			PlannerError::new(
				PlannerErrorKind::InvalidGraph,
				format!("kernel {kernel} has no lowered program"),
			)
		})?;
		let mut artifacts = LoweredArtifacts {
			chosen: &mut chosen_artifacts,
			builds: &mut artifact_builds,
			placements: &mut stage_placements,
		};
		lower_program_invocation(
			context,
			node,
			lowered,
			option.device,
			&mut state,
			&mut artifacts,
		)?;
		placements.push(KernelPlacement {
			kernel: *kernel,
			device: option.device,
		});
	}
	add_alias_dependencies(&mut state.tasks, &state.aliases)?;
	let aliases = state.aliases.clone();
	add_external_outputs(
		context.tensors,
		context.topology,
		context.discovery,
		&aliases,
		&mut state,
	)?;
	add_alias_dependencies(&mut state.tasks, &state.aliases)?;
	add_phase_barriers(&mut state.tasks);

	let scheduled = schedule(context.topology, context.discovery, &state.tasks).map_err(|error| {
		let kind = match error.kind {
			ScheduleErrorKind::DependencyCycle | ScheduleErrorKind::InvalidLifecycleDependency => {
				PlannerErrorKind::DependencyConflict
			}
			ScheduleErrorKind::ArithmeticOverflow | ScheduleErrorKind::InsufficientCapacity => {
				PlannerErrorKind::CandidateInfeasible
			}
			ScheduleErrorKind::NoRoute => PlannerErrorKind::NoRoute,
			_ => PlannerErrorKind::Schedule,
		};
		PlannerError::new(kind, error.to_string())
	})?;
	let auxiliary_peak = finalize_auxiliary_resources(
		&mut state.resources,
		&scheduled.tasks,
		&chosen_artifacts,
		&artifact_builds,
	)
	.map_err(|error| {
		if error.kind == PlannerErrorKind::ArithmeticOverflow {
			PlannerError::new(PlannerErrorKind::CandidateInfeasible, error.message)
		} else {
			error
		}
	})?;
	let (arena_objects, value_bindings) = build_arena_contract(
		&state.values,
		&scheduled.tasks,
		&state.images,
		&state.aliases,
	)?;
	let arena_layouts = pack_arenas(context.topology, &arena_objects, context.capacity)
		.map_err(|error| PlannerError::new(PlannerErrorKind::InvalidCapacity, error.to_string()))?;
	require_total_capacity(context.capacity, &arena_layouts, &auxiliary_peak)?;

	chosen_artifacts.sort_by_key(|artifact| artifact.id);
	artifact_builds.sort_by_key(|build| build.artifact);
	placements.sort();
	stage_placements.sort();
	state.logical_copies.sort();
	state.values.sort_by_key(|value| value.id);
	let mut releases = context
		.topology
		.devices
		.iter()
		.map(|device| ArenaRelease { device: device.id })
		.collect::<Vec<_>>();
	releases.sort_by_key(|release| release.device);
	let kernels = context.templates.values().cloned().collect::<Vec<_>>();
	let value_aliases = value_alias_contracts(&state.aliases)?;
	let init_images = state
		.images
		.iter()
		.map(|image| InitDataImage {
			device: image.device,
			image: image.image,
			bytes: image.bytes,
			members: image.data_members.clone(),
		})
		.collect::<Vec<_>>();
	let draft_identity = hash_draft(&DraftHashInput {
		candidate,
		topology: context.topology,
		discovery: context.discovery,
		values: &state.values,
		kernels: &kernels,
		tasks: &scheduled.tasks,
		artifacts: &chosen_artifacts,
		artifact_builds: &artifact_builds,
		resources: &state.resources,
		arena_objects: &arena_objects,
		value_bindings: &value_bindings,
		value_aliases: &value_aliases,
		init_images: &init_images,
		releases: &releases,
	});
	let draft = DraftPlan {
		identity: draft_identity,
		candidate,
		discovery: context.discovery.identity,
		topology: context.topology.identity,
		values: state.values,
		kernels,
		artifacts: chosen_artifacts,
		artifact_builds,
		tasks: scheduled.tasks,
		resources: state.resources,
		arena_objects,
		value_bindings,
		value_aliases,
		init_images,
		releases,
	};
	draft.validate(context.topology, context.discovery)
		.map_err(|errors| {
			PlannerError::new(
				PlannerErrorKind::InvalidDraft,
				format!("constructed candidate failed Draft validation: {errors}"),
			)
		})?;
	Ok(PlannedCandidate {
		draft,
		arena_layouts,
		makespan: scheduled.makespan,
		placements,
		stage_placements,
		lowered_programs: context
			.order
			.iter()
			.filter_map(|kernel| context.programs.get(kernel))
			.map(|lowered| lowered.program.clone())
			.collect(),
		value_copies: state.logical_copies,
	})
}

fn lower_program_invocation(
	context: &PlanningContext<'_>,
	node: &recipe_language::CalculationNode,
	lowered: &LoweredKernelPlan,
	device: DeviceId,
	state: &mut LoweringState,
	artifacts: &mut LoweredArtifacts<'_>,
) -> PlannerResult<()> {
	let program = &lowered.program;
	let mut dispatches = BTreeMap::new();
	for stage in &program.stages {
		let (task, submission) = state.next_submission(device)?;
		dispatches.insert(stage.id.get(), (task, submission));
	}

	let mut values = BTreeMap::<u32, ValueId>::new();
	let mut ready = BTreeMap::<u32, TaskId>::new();
	let mut writers = BTreeMap::<u32, TaskId>::new();
	let mut stage_barriers = BTreeMap::<u32, TaskId>::new();
	for buffer in &program.buffers {
		let (value, producer) = materialize_program_buffer(context, node, program, buffer, device, state)?;
		values.insert(buffer.id.get(), value);
		producer.into_iter().for_each(|producer| {
			ready.insert(buffer.id.get(), producer);
		});
	}

	for (stage_index, stage) in program.stages.iter().enumerate() {
		let stage_template = lowered.stage_templates[stage_index];
		let (task, submission) = dispatches[&stage.id.get()];
		let fault_flag = stage.fault.map(|fault| values[&fault.flag.get()]);
		let mut dependencies = stage
			.dependencies
			.iter()
			.map(|dependency| {
				stage_barriers
					.get(&dependency.get())
					.copied()
					.ok_or_else(|| {
						PlannerError::new(
							PlannerErrorKind::InvalidDraft,
							format!(
								"stage ({}, {}) depends on unavailable prior stage {}",
								program.source_kernel,
								stage.id.get(),
								dependency.get()
							),
						)
					})
			})
			.collect::<PlannerResult<Vec<_>>>()?;
		for binding in &stage.bindings {
			if primitive_access_reads(binding.mode) {
				let producer = ready.get(&binding.buffer.get()).copied().ok_or_else(|| {
					PlannerError::new(
						PlannerErrorKind::InvalidDraft,
						format!(
							"stage ({}, {}) reads buffer {} before any producer",
							program.source_kernel,
							stage.id.get(),
							binding.buffer.get()
						),
					)
				})?;
				dependencies.push(producer);
			}
		}
		if let Some(readback) = fault_flag.and_then(|flag| state.fault_readbacks.get(&flag).copied()) {
			dependencies.push(readback);
		}
		dependencies.retain(|dependency| *dependency != task);
		dependencies.sort();
		dependencies.dedup();

		let build_bindings = stage
			.bindings
			.iter()
			.map(|binding| lower_build_binding(binding, &values))
			.collect::<PlannerResult<Vec<_>>>()?;
		let inputs = build_bindings
			.iter()
			.filter(|binding| fault_flag != Some(binding.value) && binding.access.reads())
			.map(|binding| binding.value)
			.collect::<Vec<_>>();
		let outputs = build_bindings
			.iter()
			.filter(|binding| fault_flag != Some(binding.value) && binding.access.writes())
			.map(|binding| binding.value)
			.collect::<Vec<_>>();
		let artifact = ArtifactId::new(stage_template.get());
		let mut build = ArtifactBuildRecipe {
			artifact,
			kernel_template: stage_template,
			source_kernel: program.source_kernel,
			provenance: ArtifactBuildProvenance {
				program_digest: Digest::new(program.digest.bytes()),
				stage_ordinal: stage.id.get(),
				contract_digest: Digest::ZERO,
			},
			bindings: build_bindings,
			dispatch: ArtifactDispatchGeometry {
				logical_lanes: stage.geometry.logical_lanes,
				workgroup_lanes: stage.geometry.workgroup_lanes,
				workgroups: stage.geometry.workgroups,
			},
			work: ArtifactWorkBounds {
				flops: stage.resources.flops,
				integer_operations: stage.resources.integer_operations,
				atomic_operations: stage.resources.atomic_operations,
			},
			fault_flag,
			resources: KernelResourceBounds {
				private_bytes_per_lane: stage.resources.private_bytes_per_lane,
				shared_bytes_per_workgroup: stage.resources.shared_bytes_per_workgroup,
				scratch_bytes_per_dispatch: ByteCount::ZERO,
				maximum_workgroup_lanes: stage.resources.maximum_workgroup_lanes,
			},
		};
		build.provenance.contract_digest = hash_build_contract(&build);
		build.validate().map_err(|errors| {
			PlannerError::new(
				PlannerErrorKind::InvalidDraft,
				format!(
					"stage ({}, {}) produced an invalid build recipe: {errors}",
					program.source_kernel,
					stage.id.get()
				),
			)
		})?;
		select_or_defer_artifact(context, device, &build, artifacts.chosen, artifacts.builds)?;
		state.tasks.push(UnscheduledTask {
			id: task,
			phase: RunPhase::Loop,
			dependencies,
			kind: TaskKind::Calculation(CalculationTask {
				device,
				kernel_template: stage_template,
				artifact,
				inputs,
				outputs,
				fault_flag,
				work: stage.resources.flops,
				submission,
			}),
		});
		let completion_barrier = match fault_flag {
			Some(fault_flag) => {
				let readback = state.next_task()?;
				let metric = MetricId::new(readback.get());
				let slot = MetricSlotId::new(readback.get());
				state.resources
					.metrics
					.push(MetricSlot { id: slot, metric });
				state.tasks.push(UnscheduledTask {
					id: readback,
					phase: RunPhase::Loop,
					dependencies: vec![task],
					kind: TaskKind::Metric(MetricTask {
						purpose: MetricPurpose::FaultReadback { calculation: task },
						metric,
						value: fault_flag,
						slot,
					}),
				});
				state.fault_readbacks.insert(fault_flag, readback);
				readback
			}
			None => task,
		};
		stage_barriers.insert(stage.id.get(), completion_barrier);
		for binding in &stage.bindings {
			if primitive_access_writes(binding.mode) {
				ready.insert(binding.buffer.get(), completion_barrier);
				writers.insert(binding.buffer.get(), task);
				let buffer = &program.buffers[usize::try_from(binding.buffer.get()).map_err(|error| {
					PlannerError::new(
						PlannerErrorKind::ArithmeticOverflow,
						format!("buffer index conversion failed: {error}"),
					)
				})?];
				if buffer.lifetime != BufferLifetime::ProgramFaultFlag {
					set_value_producer(state, values[&binding.buffer.get()], task)?;
				}
			}
		}
		artifacts.placements.push(StagePlacement {
			source_kernel: program.source_kernel,
			stage_ordinal: stage.id.get(),
			kernel_template: stage_template,
			device,
			artifact,
		});
	}

	let source_inputs = node
		.kernel
		.inputs
		.iter()
		.map(|logical| program_tensor_value(program, *logical, &values))
		.collect::<PlannerResult<Vec<_>>>()?;
	let source_outputs = node
		.kernel
		.outputs
		.iter()
		.map(|logical| program_tensor_value(program, *logical, &values))
		.collect::<PlannerResult<Vec<_>>>()?;
	for (logical, physical) in node.kernel.outputs.iter().zip(&source_outputs) {
		let buffer = program_tensor_buffer(program, *logical)?;
		let writer = match writers.get(&buffer.id.get()).copied() {
			Some(writer) => writer,
			None => {
				if buffer.access.storage_bytes != ByteCount::ZERO {
					return Err(PlannerError::new(
						PlannerErrorKind::InvalidDraft,
						format!(
							"nonempty output {logical} of kernel {} has no dispatch writer",
							program.source_kernel
						),
					));
				}
				state.init_tasks[&device]
			}
		};
		let readiness = match ready.get(&buffer.id.get()).copied() {
			Some(readiness) => readiness,
			None => state.init_tasks[&device],
		};
		set_value_producer(state, *physical, writer)?;
		state.push_copy(
			*logical,
			RuntimeCopy {
				device,
				value: *physical,
				producer: readiness,
			},
		)?;
	}
	let rules = program
		.source_aliases
		.iter()
		.map(|rule| {
			Ok(AliasRule {
				input: recipe_core::KernelInputId::new(stable_argument_position(rule.input)?),
				output: recipe_core::KernelOutputId::new(stable_argument_position(rule.output)?),
				permission: rule.permission,
			})
		})
		.collect::<PlannerResult<Vec<_>>>()?;
	validate_source_must_alias(program, node)?;
	let alias_task = first_output_writer(program, node, &dispatches)?.unwrap_or(state.init_tasks[&device]);
	state.aliases.push(AliasInvocation {
		task: alias_task,
		inputs: source_inputs,
		outputs: source_outputs,
		rules,
	});
	Ok(())
}

fn materialize_program_buffer(
	context: &PlanningContext<'_>,
	node: &recipe_language::CalculationNode,
	program: &LoweredProgram,
	buffer: &ProgramBuffer,
	device: DeviceId,
	state: &mut LoweringState,
) -> PlannerResult<(ValueId, Option<TaskId>)> {
	match buffer.origin {
		BufferOrigin::Tensor(logical) if node.kernel.inputs.contains(&logical) => {
			let copy = ensure_copy(
				logical,
				device,
				context.tensors,
				context.topology,
				context.discovery,
				state,
			)?;
			Ok((copy.value, Some(copy.producer)))
		}
		BufferOrigin::Tensor(logical) if node.kernel.outputs.contains(&logical) => {
			let value = state.next_value()?;
			state.values.push(ValueSpec {
				id: value,
				dtype: buffer.dtype,
				bytes: buffer.access.storage_bytes,
				device,
				producer: None,
			});
			Ok((value, None))
		}
		BufferOrigin::Tensor(logical) => Err(PlannerError::new(
			PlannerErrorKind::InvalidDraft,
			format!(
				"lowered program {} contains unrelated tensor buffer {logical}",
				program.source_kernel
			),
		)),
		BufferOrigin::Scratch { .. } => {
			let value = state.next_value()?;
			state.values.push(ValueSpec {
				id: value,
				dtype: buffer.dtype,
				bytes: buffer.access.storage_bytes,
				device,
				producer: None,
			});
			Ok((value, None))
		}
		BufferOrigin::FaultFlag => {
			let value = state
				.fault_flags
				.get(&(program.source_kernel, buffer.id.get()))
				.copied()
				.ok_or_else(|| {
					PlannerError::new(
						PlannerErrorKind::InvalidDraft,
						format!(
							"fault buffer {} for kernel {} is absent from its init image",
							buffer.id.get(),
							program.source_kernel
						),
					)
				})?;
			Ok((value, Some(state.init_tasks[&device])))
		}
	}
}

fn lower_build_binding(
	binding: &recipe_primitives::BufferBinding,
	values: &BTreeMap<u32, ValueId>,
) -> PlannerResult<ArtifactBuildBinding> {
	let value = values.get(&binding.buffer.get()).copied().ok_or_else(|| {
		PlannerError::new(
			PlannerErrorKind::InvalidDraft,
			format!(
				"stage binding references absent buffer {}",
				binding.buffer.get()
			),
		)
	})?;
	Ok(ArtifactBuildBinding {
		value,
		dtype: binding.dtype,
		access: match binding.mode {
			AccessMode::Read => ArtifactBuildAccess::Read,
			AccessMode::Write => ArtifactBuildAccess::Write,
			AccessMode::ReadWrite => ArtifactBuildAccess::ReadWrite,
			AccessMode::ReadWriteAtomic => ArtifactBuildAccess::ReadWriteAtomic,
		},
		view: ArtifactBuildView {
			logical_extents: binding.view.logical_extents.clone(),
			offset_elements: binding.view.offset_elements,
			strides: binding.view.strides.clone(),
			storage_bytes: binding.view.storage_bytes,
		},
	})
}

fn select_or_defer_artifact(
	context: &PlanningContext<'_>,
	device: DeviceId,
	build: &ArtifactBuildRecipe,
	chosen_artifacts: &mut Vec<ArtifactIdentity>,
	artifact_builds: &mut Vec<ArtifactBuildRecipe>,
) -> PlannerResult<()> {
	let catalog_entry = context
		.artifacts
		.iter()
		.find(|artifact| artifact.id == build.artifact);
	match catalog_entry {
		Some(artifact) => {
			let target_matches = context
				.discovery
				.device(device)
				.and_then(|discovered| discovered.calculation.as_ref())
				.is_some_and(|capability| capability.target == artifact.target);
			if artifact.kernel_template != build.kernel_template
				|| artifact.build != Some(build.provenance)
				|| artifact.resources != build.resources
				|| !target_matches
			{
				return Err(PlannerError::new(
					PlannerErrorKind::InvalidArtifact,
					format!(
						"catalog artifact {} collides with a deferred stage but does not exactly realize it",
						artifact.id
					),
				));
			}
			chosen_artifacts.push(artifact.clone());
		}
		None => artifact_builds.push(build.clone()),
	}
	Ok(())
}

fn set_value_producer(state: &mut LoweringState, value: ValueId, producer: TaskId) -> PlannerResult<()> {
	let spec = state
		.values
		.iter_mut()
		.find(|candidate| candidate.id == value)
		.ok_or_else(|| {
			PlannerError::new(
				PlannerErrorKind::InvalidDraft,
				format!("value {value} is absent while assigning its producer"),
			)
		})?;
	spec.producer = Some(producer);
	Ok(())
}

fn program_tensor_buffer(program: &LoweredProgram, logical: ValueId) -> PlannerResult<&ProgramBuffer> {
	program
		.buffers
		.iter()
		.find(|buffer| buffer.origin == BufferOrigin::Tensor(logical))
		.ok_or_else(|| {
			PlannerError::new(
				PlannerErrorKind::InvalidDraft,
				format!(
					"kernel {} has no program buffer for tensor {logical}",
					program.source_kernel
				),
			)
		})
}

fn program_tensor_value(
	program: &LoweredProgram,
	logical: ValueId,
	values: &BTreeMap<u32, ValueId>,
) -> PlannerResult<ValueId> {
	let buffer = program_tensor_buffer(program, logical)?;
	values.get(&buffer.id.get()).copied().ok_or_else(|| {
		PlannerError::new(
			PlannerErrorKind::InvalidDraft,
			format!(
				"kernel {} tensor {logical} has no materialized value",
				program.source_kernel
			),
		)
	})
}

fn first_output_writer(
	program: &LoweredProgram,
	node: &recipe_language::CalculationNode,
	dispatches: &BTreeMap<u32, (TaskId, SubmissionSlots)>,
) -> PlannerResult<Option<TaskId>> {
	let output_buffers = (0..node.kernel.outputs.len())
		.map(|index| source_buffer(program, node, false, index).map(|buffer| buffer.id))
		.collect::<PlannerResult<BTreeSet<_>>>()?;
	Ok(program.stages.iter().find_map(|stage| {
		stage.bindings
			.iter()
			.any(|binding| output_buffers.contains(&binding.buffer) && primitive_access_writes(binding.mode))
			.then(|| dispatches[&stage.id.get()].0)
	}))
}

const fn primitive_access_reads(access: AccessMode) -> bool {
	matches!(
		access,
		AccessMode::Read | AccessMode::ReadWrite | AccessMode::ReadWriteAtomic
	)
}

const fn primitive_access_writes(access: AccessMode) -> bool {
	matches!(
		access,
		AccessMode::Write | AccessMode::ReadWrite | AccessMode::ReadWriteAtomic
	)
}

fn stable_argument_position(index: usize) -> PlannerResult<u64> {
	u64::try_from(index)
		.ok()
		.and_then(|value| value.checked_add(1))
		.ok_or_else(|| {
			PlannerError::new(
				PlannerErrorKind::ArithmeticOverflow,
				"kernel argument position overflowed u64",
			)
		})
}

fn validate_source_must_alias(program: &LoweredProgram, node: &recipe_language::CalculationNode) -> PlannerResult<()> {
	for rule in &program.source_aliases {
		if rule.permission != AliasPermission::MustAliasExact {
			continue;
		}
		let input = source_buffer(program, node, true, rule.input)?;
		let output = source_buffer(program, node, false, rule.output)?;
		if input.dtype != output.dtype || input.access != output.access {
			return Err(PlannerError::new(
				PlannerErrorKind::InvalidGraph,
				format!(
					"kernel {} requires exact alias {} -> {}, but their typed static views differ",
					program.source_kernel, rule.input, rule.output
				),
			));
		}
	}
	Ok(())
}

fn source_buffer<'program>(
	program: &'program LoweredProgram,
	node: &recipe_language::CalculationNode,
	input: bool,
	index: usize,
) -> PlannerResult<&'program ProgramBuffer> {
	let logical = match input {
		true => node.kernel.inputs.get(index),
		false => node.kernel.outputs.get(index),
	}
	.copied()
	.ok_or_else(|| {
		PlannerError::new(
			PlannerErrorKind::InvalidGraph,
			"source alias index exceeds its declared arity",
		)
	})?;
	program_tensor_buffer(program, logical)
}

fn value_alias_contracts(aliases: &[AliasInvocation]) -> PlannerResult<Vec<ValueAliasContract>> {
	let mut contracts = Vec::new();
	for invocation in aliases {
		for rule in &invocation.rules {
			let input_index = usize::try_from(rule.input.get().saturating_sub(1)).map_err(|error| {
				PlannerError::new(
					PlannerErrorKind::ArithmeticOverflow,
					format!("alias input index conversion failed: {error}"),
				)
			})?;
			let output_index = usize::try_from(rule.output.get().saturating_sub(1)).map_err(|error| {
				PlannerError::new(
					PlannerErrorKind::ArithmeticOverflow,
					format!("alias output index conversion failed: {error}"),
				)
			})?;
			contracts.push(ValueAliasContract {
				input: invocation.inputs[input_index],
				output: invocation.outputs[output_index],
				permission: rule.permission,
			});
		}
	}
	contracts.sort_by_key(|contract| (contract.input, contract.output));
	Ok(contracts)
}

fn hash_build_contract(build: &ArtifactBuildRecipe) -> Digest {
	let mut digest = StableDigest::new("recipe-planner-artifact-build-v1");
	digest.u64(build.artifact.get());
	digest.u64(build.kernel_template.get());
	digest.u64(build.source_kernel.get());
	digest.digest(build.provenance.program_digest);
	digest.u64(u64::from(build.provenance.stage_ordinal));
	digest.length(build.bindings.len());
	for binding in &build.bindings {
		digest.u64(binding.value.get());
		digest.u8(dtype_tag(binding.dtype));
		digest.u8(match binding.access {
			ArtifactBuildAccess::Read => 0,
			ArtifactBuildAccess::Write => 1,
			ArtifactBuildAccess::ReadWrite => 2,
			ArtifactBuildAccess::ReadWriteAtomic => 3,
		});
		digest.length(binding.view.logical_extents.len());
		for extent in &binding.view.logical_extents {
			digest.u64(*extent);
		}
		digest.u64(binding.view.offset_elements);
		digest.length(binding.view.strides.len());
		for stride in &binding.view.strides {
			digest.u64(*stride);
		}
		digest.u64(binding.view.storage_bytes.get());
	}
	digest.u64(build.dispatch.logical_lanes);
	digest.u64(u64::from(build.dispatch.workgroup_lanes));
	digest.u64(build.dispatch.workgroups);
	digest.u64(build.work.flops.get());
	digest.u64(build.work.integer_operations);
	digest.u64(build.work.atomic_operations);
	match build.fault_flag {
		Some(fault) => {
			digest.u8(1);
			digest.u64(fault.get());
		}
		None => digest.u8(0),
	}
	digest.u64(build.resources.private_bytes_per_lane.get());
	digest.u64(build.resources.shared_bytes_per_workgroup.get());
	digest.u64(build.resources.scratch_bytes_per_dispatch.get());
	digest.u64(u64::from(build.resources.maximum_workgroup_lanes));
	digest.finish()
}

fn add_alias_dependencies(tasks: &mut [UnscheduledTask], aliases: &[AliasInvocation]) -> PlannerResult<()> {
	let mut dependencies = BTreeMap::<TaskId, Vec<TaskId>>::new();
	for invocation in aliases {
		for rule in &invocation.rules {
			if rule.permission != AliasPermission::MustAliasExact {
				continue;
			}
			let input_index = usize::try_from(rule.input.get().saturating_sub(1)).map_err(|_| {
				PlannerError::new(
					PlannerErrorKind::ArithmeticOverflow,
					"must-alias input index does not fit usize",
				)
			})?;
			let input = invocation.inputs.get(input_index).copied().ok_or_else(|| {
				PlannerError::new(
					PlannerErrorKind::InvalidDraft,
					"must-alias input index is absent from invocation",
				)
			})?;
			for consumer in tasks
				.iter()
				.filter(|task| task.id != invocation.task && task_kind_references(&task.kind, input))
			{
				if consumer.phase == RunPhase::Exit {
					return Err(PlannerError::new(
						PlannerErrorKind::DependencyConflict,
						format!(
							"task {} must overwrite value {input}, but exit task {} still needs the old value",
							invocation.task, consumer.id
						),
					));
				}
				dependencies
					.entry(invocation.task)
					.or_default()
					.push(consumer.id);
			}
		}
	}
	for task in tasks {
		if let Some(required) = dependencies.get(&task.id) {
			task.dependencies.extend(required);
			task.dependencies.sort();
			task.dependencies.dedup();
		}
	}
	Ok(())
}

fn add_phase_barriers(tasks: &mut [UnscheduledTask]) {
	let init = tasks
		.iter()
		.filter_map(|task| (task.phase == RunPhase::Init).then_some(task.id))
		.collect::<Vec<_>>();
	let loop_tasks = tasks
		.iter()
		.filter_map(|task| (task.phase == RunPhase::Loop).then_some(task.id))
		.collect::<Vec<_>>();
	for task in tasks {
		match task.phase {
			RunPhase::Init => continue,
			RunPhase::Loop => task.dependencies.extend(&init),
			RunPhase::Exit => {
				task.dependencies.extend(&init);
				task.dependencies.extend(&loop_tasks);
			}
		}
		task.dependencies.sort();
		task.dependencies.dedup();
	}
}

fn trial_timing(
	topology: &Topology,
	discovery: &DiscoveryProfile,
	tasks: &[UnscheduledTask],
	trial: UnscheduledTask,
) -> PlannerResult<Option<(Nanoseconds, Nanoseconds)>> {
	let trial_id = trial.id;
	trial_chain_timing(topology, discovery, tasks, &[trial], trial_id)
}

fn trial_chain_timing(
	topology: &Topology,
	discovery: &DiscoveryProfile,
	tasks: &[UnscheduledTask],
	trials: &[UnscheduledTask],
	completion: TaskId,
) -> PlannerResult<Option<(Nanoseconds, Nanoseconds)>> {
	let mut tasks = tasks.to_vec();
	tasks.extend_from_slice(trials);
	add_phase_barriers(&mut tasks);
	let scheduled = match schedule(topology, discovery, &tasks) {
		Ok(scheduled) => scheduled,
		Err(error)
			if matches!(
				error.kind,
				ScheduleErrorKind::ArithmeticOverflow
					| ScheduleErrorKind::InsufficientCapacity
					| ScheduleErrorKind::NoRoute
			) =>
		{
			return Ok(None);
		}
		Err(error)
			if matches!(
				error.kind,
				ScheduleErrorKind::DependencyCycle | ScheduleErrorKind::InvalidLifecycleDependency
			) =>
		{
			return Err(PlannerError::new(
				PlannerErrorKind::DependencyConflict,
				error.to_string(),
			));
		}
		Err(error) => {
			return Err(PlannerError::new(
				PlannerErrorKind::Schedule,
				error.to_string(),
			));
		}
	};
	let end = scheduled
		.tasks
		.iter()
		.find(|task| task.id == completion)
		.map(|task| task.window.end)
		.ok_or_else(|| {
			PlannerError::new(
				PlannerErrorKind::Schedule,
				format!("trial task {completion} is absent from its static schedule"),
			)
		})?;
	Ok(Some((end, scheduled.makespan)))
}

fn directed_routes(topology: &Topology, source: DeviceId, destination: DeviceId) -> Vec<Vec<LinkId>> {
	if source == destination {
		return vec![Vec::new()];
	}
	let mut outgoing = BTreeMap::<DeviceId, Vec<(LinkId, DeviceId)>>::new();
	for link in &topology.links {
		outgoing
			.entry(link.from)
			.or_default()
			.push((link.id, link.to));
	}
	for links in outgoing.values_mut() {
		links.sort();
	}
	let mut routes = Vec::new();
	let mut visited = BTreeSet::from([source]);
	let mut path = Vec::new();
	enumerate_directed_routes(
		source,
		destination,
		&outgoing,
		&mut visited,
		&mut path,
		&mut routes,
	);
	routes
}

fn enumerate_directed_routes(
	current: DeviceId,
	destination: DeviceId,
	outgoing: &BTreeMap<DeviceId, Vec<(LinkId, DeviceId)>>,
	visited: &mut BTreeSet<DeviceId>,
	path: &mut Vec<LinkId>,
	routes: &mut Vec<Vec<LinkId>>,
) {
	for (link, next) in outgoing.get(&current).into_iter().flatten() {
		if !visited.insert(*next) {
			continue;
		}
		path.push(*link);
		if *next == destination {
			routes.push(path.clone());
		} else {
			enumerate_directed_routes(*next, destination, outgoing, visited, path, routes);
		}
		path.pop();
		visited.remove(next);
	}
}

fn stable_offset(base: u64, offset: usize, kind: &str) -> PlannerResult<u64> {
	let offset = u64::try_from(offset).map_err(|error| {
		PlannerError::new(
			PlannerErrorKind::ArithmeticOverflow,
			format!("{kind} offset does not fit u64: {error}"),
		)
	})?;
	base.checked_add(offset).ok_or_else(|| {
		PlannerError::new(
			PlannerErrorKind::ArithmeticOverflow,
			format!("{kind} ID space overflowed while lowering a transfer route"),
		)
	})
}

fn build_transfer_chain(
	source: RuntimeCopy,
	destination: DeviceId,
	route: &[LinkId],
	dtype: DType,
	bytes: ByteCount,
	topology: &Topology,
	allocation: TransferAllocationStart,
) -> PlannerResult<TransferChain> {
	topology
		.validate_route(source.device, destination, route)
		.map_err(|errors| {
			PlannerError::new(
				PlannerErrorKind::InvalidDraft,
				format!(
					"route from {} to {destination} became invalid while lowering transfer hops: {errors}",
					source.device
				),
			)
		})?;

	let mut endpoints = Vec::with_capacity(route.len().max(1));
	if route.is_empty() {
		endpoints.push((source.device, destination, None));
	} else {
		let mut current = source.device;
		for link_id in route {
			let link = topology.link(*link_id).ok_or_else(|| {
				PlannerError::new(
					PlannerErrorKind::InvalidDraft,
					format!("transfer route references absent link {link_id}"),
				)
			})?;
			if link.from != current {
				return Err(PlannerError::new(
					PlannerErrorKind::InvalidDraft,
					format!(
						"transfer hop {link_id} starts at {}, expected {current}",
						link.from
					),
				));
			}
			endpoints.push((current, link.to, Some(*link_id)));
			current = link.to;
		}
		if current != destination {
			return Err(PlannerError::new(
				PlannerErrorKind::InvalidDraft,
				format!("transfer route ends at {current}, expected {destination}"),
			));
		}
	}

	let final_value = ValueId::new(allocation.value);
	let mut task_ids = Vec::with_capacity(endpoints.len());
	for hop in 0..endpoints.len() {
		task_ids.push(TaskId::new(stable_offset(allocation.task, hop, "task")?));
	}
	let final_task = *task_ids.last().ok_or_else(|| {
		PlannerError::new(
			PlannerErrorKind::InvalidDraft,
			"transfer route did not produce a hop task",
		)
	})?;

	let mut hop_values = Vec::with_capacity(endpoints.len());
	for hop in 0..endpoints.len() {
		let value = if hop + 1 == endpoints.len() {
			final_value
		} else {
			ValueId::new(stable_offset(allocation.value, hop + 1, "value")?)
		};
		hop_values.push(value);
	}

	// Keep the logical destination at the allocator head. Intermediate values
	// follow it in route order, even though their producers execute first.
	let mut values = Vec::with_capacity(endpoints.len());
	values.push(ValueSpec {
		id: final_value,
		dtype,
		bytes,
		device: destination,
		producer: Some(final_task),
	});
	for (hop, (_, hop_destination, _)) in endpoints
		.iter()
		.copied()
		.enumerate()
		.take(endpoints.len().saturating_sub(1))
	{
		values.push(ValueSpec {
			id: hop_values[hop],
			dtype,
			bytes,
			device: hop_destination,
			producer: Some(task_ids[hop]),
		});
	}

	let mut tasks = Vec::with_capacity(endpoints.len());
	let mut submission_devices = Vec::with_capacity(endpoints.len());
	let mut source_value = source.value;
	for (hop, (hop_source, hop_destination, link)) in endpoints.iter().copied().enumerate() {
		let task = task_ids[hop];
		let destination_value = hop_values[hop];
		let submission = SubmissionSlots {
			queue: QueueSlotId::new(task.get()),
			completion: CompletionSlotId::new(task.get()),
		};
		tasks.push(UnscheduledTask {
			id: task,
			phase: RunPhase::Loop,
			dependencies: vec![if hop == 0 {
				source.producer
			} else {
				task_ids[hop - 1]
			}],
			kind: TaskKind::Transfer(TransferTask {
				source: TransferEndpoint::Device {
					device: hop_source,
					value: source_value,
				},
				destination: TransferEndpoint::Device {
					device: hop_destination,
					value: destination_value,
				},
				bytes,
				route: link.into_iter().collect(),
				lane_claims: Vec::new(),
				submission,
			}),
		});
		submission_devices.push(hop_source);
		source_value = destination_value;
	}

	Ok(TransferChain {
		final_value,
		final_task,
		values,
		tasks,
		submission_devices,
	})
}

fn initialize_data_images(
	order: &[KernelTemplateId],
	nodes: &BTreeMap<KernelTemplateId, &recipe_language::CalculationNode>,
	tensors: &BTreeMap<ValueId, &Tensor>,
	programs: &BTreeMap<KernelTemplateId, LoweredKernelPlan>,
	topology: &Topology,
	selected: &BTreeMap<KernelTemplateId, &PlacementOption>,
	state: &mut LoweringState,
) -> PlannerResult<()> {
	let mut image_tensors = BTreeMap::<DeviceId, BTreeSet<ValueId>>::new();
	let mut fault_buffers = BTreeMap::<DeviceId, Vec<(KernelTemplateId, u32)>>::new();
	for kernel in order {
		let node = nodes.get(kernel).ok_or_else(|| {
			PlannerError::new(
				PlannerErrorKind::InvalidGraph,
				format!("kernel {kernel} is absent while building data images"),
			)
		})?;
		let program = programs.get(kernel).ok_or_else(|| {
			PlannerError::new(
				PlannerErrorKind::InvalidGraph,
				format!("kernel {kernel} has no lowered program while building data images"),
			)
		})?;
		for buffer in &program.program.buffers {
			if buffer.lifetime == BufferLifetime::ProgramFaultFlag {
				fault_buffers
					.entry(selected[kernel].device)
					.or_default()
					.push((*kernel, buffer.id.get()));
			}
		}
		for input in &node.kernel.inputs {
			if tensors
				.get(input)
				.is_some_and(|tensor| tensor.external_input)
			{
				image_tensors
					.entry(selected[kernel].device)
					.or_default()
					.insert(*input);
			}
		}
	}
	let mut devices = topology
		.devices
		.iter()
		.map(|device| device.id)
		.collect::<Vec<_>>();
	devices.sort();
	for tensor in tensors
		.values()
		.copied()
		.filter(|tensor| tensor.external_input && tensor.external_output)
	{
		if !image_tensors
			.values()
			.any(|values| values.contains(&tensor.id))
		{
			let device = devices.first().copied().ok_or_else(|| {
				PlannerError::new(
					PlannerErrorKind::InvalidTopology,
					"topology has no device for pass-through external tensor",
				)
			})?;
			image_tensors.entry(device).or_default().insert(tensor.id);
		}
	}

	for device in devices {
		let (task, submission) = state.next_submission(device)?;
		state.init_tasks.insert(device, task);
		let image_value = state.next_value()?;
		let mut offset = ByteCount::ZERO;
		let mut external_members = Vec::new();
		let mut data_members = Vec::new();
		for logical in image_tensors.get(&device).into_iter().flatten() {
			let tensor = tensors.get(logical).ok_or_else(|| {
				PlannerError::new(
					PlannerErrorKind::InvalidGraph,
					format!("data-image tensor {logical} is absent"),
				)
			})?;
			let member_offset = ByteOffset::new(offset.get());
			let physical = state.next_value()?;
			state.values.push(ValueSpec {
				id: physical,
				dtype: tensor.dtype,
				bytes: tensor.storage_bytes,
				device,
				producer: Some(task),
			});
			state.push_copy(
				*logical,
				RuntimeCopy {
					device,
					value: physical,
					producer: task,
				},
			)?;
			external_members.push((physical, member_offset));
			data_members.push(InitDataImageMember {
				logical: *logical,
				physical,
				dtype: tensor.dtype,
				bytes: tensor.storage_bytes,
				image_offset: member_offset,
			});
			offset = offset.checked_add(tensor.storage_bytes).ok_or_else(|| {
				PlannerError::new(
					PlannerErrorKind::ArithmeticOverflow,
					"data-image byte size overflowed",
				)
			})?;
		}
		for (kernel, buffer) in fault_buffers.get(&device).into_iter().flatten() {
			let fault_flag = state.next_value()?;
			state.values.push(ValueSpec {
				id: fault_flag,
				dtype: DType::I32,
				bytes: ByteCount::new(4),
				device,
				producer: Some(task),
			});
			external_members.push((fault_flag, ByteOffset::new(offset.get())));
			offset = offset.checked_add(ByteCount::new(4)).ok_or_else(|| {
				PlannerError::new(
					PlannerErrorKind::ArithmeticOverflow,
					"fault-flag data-image size overflowed",
				)
			})?;
			if state
				.fault_flags
				.insert((*kernel, *buffer), fault_flag)
				.is_some()
			{
				return Err(PlannerError::new(
					PlannerErrorKind::InvalidDraft,
					format!("kernel {kernel} buffer {buffer} has more than one fault flag"),
				));
			}
		}
		let image_bytes = offset.max(ByteCount::new(4));
		state.values.push(ValueSpec {
			id: image_value,
			dtype: DType::I32,
			bytes: image_bytes,
			device,
			producer: Some(task),
		});
		let mut members = vec![(image_value, ByteOffset::new(0))];
		members.extend(external_members);
		state.images.push(ImageRegion {
			device,
			image: image_value,
			bytes: image_bytes,
			members,
			data_members,
		});
		state.tasks.push(UnscheduledTask {
			id: task,
			phase: RunPhase::Init,
			dependencies: Vec::new(),
			kind: TaskKind::Transfer(TransferTask {
				source: TransferEndpoint::External,
				destination: TransferEndpoint::Device {
					device,
					value: image_value,
				},
				bytes: image_bytes,
				route: Vec::new(),
				lane_claims: Vec::new(),
				submission,
			}),
		});
	}
	Ok(())
}

fn ensure_copy(
	logical: ValueId,
	destination: DeviceId,
	tensors: &BTreeMap<ValueId, &Tensor>,
	topology: &Topology,
	discovery: &DiscoveryProfile,
	state: &mut LoweringState,
) -> PlannerResult<RuntimeCopy> {
	if let Some(copy) = state
		.copies
		.get(&logical)
		.and_then(|copies| copies.get(&destination))
		.copied()
	{
		return Ok(copy);
	}
	let tensor = tensors.get(&logical).ok_or_else(|| {
		PlannerError::new(
			PlannerErrorKind::InvalidGraph,
			format!("tensor {logical} is absent while inserting a transfer"),
		)
	})?;
	let sources = state
		.copies
		.get(&logical)
		.map(|copies| copies.values().copied().collect::<Vec<_>>())
		.unwrap_or_default();
	let task_start = state.task_ids.peek();
	let value_start = state.value_ids.peek();
	let allocation = TransferAllocationStart {
		task: task_start,
		value: value_start,
	};
	let mut best: Option<(
		Nanoseconds,
		Nanoseconds,
		DeviceId,
		ValueId,
		Vec<LinkId>,
		RuntimeCopy,
	)> = None;
	let mut route_count = 0_usize;
	let mut schedulable_count = 0_usize;
	for source in sources {
		for route in directed_routes(topology, source.device, destination) {
			route_count = route_count.saturating_add(1);
			let chain = build_transfer_chain(
				source,
				destination,
				&route,
				tensor.dtype,
				tensor.storage_bytes,
				topology,
				allocation,
			)?;
			let Some((end, makespan)) = trial_chain_timing(
				topology,
				discovery,
				&state.tasks,
				&chain.tasks,
				chain.final_task,
			)?
			else {
				continue;
			};
			schedulable_count = schedulable_count.saturating_add(1);
			let key = (end, makespan, source.device, source.value, route, source);
			if best.as_ref().is_none_or(|known| {
				(&key.0, &key.1, &key.2, &key.3, &key.4) < (&known.0, &known.1, &known.2, &known.3, &known.4)
			}) {
				best = Some(key);
			}
		}
	}
	let (_, _, _, _, route, source) = best.ok_or_else(|| {
		let kind = if route_count == 0 {
			PlannerErrorKind::NoRoute
		} else {
			PlannerErrorKind::CandidateInfeasible
		};
		PlannerError::new(
			kind,
			format!(
				"tensor {logical} has {route_count} directed routes to device {destination}, but {schedulable_count} are schedulable"
			),
		)
	})?;
	let chain = build_transfer_chain(
		source,
		destination,
		&route,
		tensor.dtype,
		tensor.storage_bytes,
		topology,
		allocation,
	)?;
	for expected in &chain.values {
		let allocated = state.next_value()?;
		if allocated != expected.id {
			return Err(PlannerError::new(
				PlannerErrorKind::InvalidDraft,
				"stable planner value ID allocation changed during transfer search",
			));
		}
	}
	for (expected, source_device) in chain.tasks.iter().zip(&chain.submission_devices) {
		let (allocated_task, allocated_submission) = state.next_submission(*source_device)?;
		let TaskKind::Transfer(transfer) = &expected.kind else {
			return Err(PlannerError::new(
				PlannerErrorKind::InvalidDraft,
				"transfer chain contains a non-transfer task",
			));
		};
		if allocated_task != expected.id || allocated_submission != transfer.submission {
			return Err(PlannerError::new(
				PlannerErrorKind::InvalidDraft,
				"stable planner task ID allocation changed during transfer search",
			));
		}
	}
	let copy = RuntimeCopy {
		device: destination,
		value: chain.final_value,
		producer: chain.final_task,
	};
	state.values.extend(chain.values);
	state.tasks.extend(chain.tasks);
	state.push_copy(logical, copy)?;
	Ok(copy)
}

fn must_alias_inputs(aliases: &[AliasInvocation]) -> PlannerResult<BTreeSet<ValueId>> {
	let mut values = BTreeSet::new();
	for invocation in aliases {
		for rule in &invocation.rules {
			if rule.permission != AliasPermission::MustAliasExact {
				continue;
			}
			let input_index = usize::try_from(rule.input.get().saturating_sub(1)).map_err(|_| {
				PlannerError::new(
					PlannerErrorKind::ArithmeticOverflow,
					"must-alias input index does not fit usize",
				)
			})?;
			let input = invocation.inputs.get(input_index).copied().ok_or_else(|| {
				PlannerError::new(
					PlannerErrorKind::InvalidDraft,
					"must-alias input index is absent from invocation",
				)
			})?;
			values.insert(input);
		}
	}
	Ok(values)
}

fn add_external_outputs(
	tensors: &BTreeMap<ValueId, &Tensor>,
	topology: &Topology,
	discovery: &DiscoveryProfile,
	aliases: &[AliasInvocation],
	state: &mut LoweringState,
) -> PlannerResult<()> {
	let overwritten = must_alias_inputs(aliases)?;
	let mut outputs = tensors
		.values()
		.copied()
		.filter(|tensor| tensor.external_output)
		.collect::<Vec<_>>();
	outputs.sort_by_key(|tensor| tensor.id);
	for tensor in outputs {
		let copies = state.copies.get(&tensor.id).ok_or_else(|| {
			PlannerError::new(
				PlannerErrorKind::InvalidDraft,
				format!("external output {} has no resident copy", tensor.id),
			)
		})?;
		let task = TaskId::new(state.task_ids.peek());
		let submission = SubmissionSlots {
			queue: QueueSlotId::new(task.get()),
			completion: CompletionSlotId::new(task.get()),
		};
		let mut best: Option<(Nanoseconds, Nanoseconds, DeviceId, ValueId, RuntimeCopy)> = None;
		let mut safe_count = 0_usize;
		for source in copies
			.values()
			.copied()
			.filter(|copy| !overwritten.contains(&copy.value))
		{
			safe_count = safe_count.saturating_add(1);
			let trial = UnscheduledTask {
				id: task,
				phase: RunPhase::Exit,
				dependencies: vec![source.producer],
				kind: TaskKind::Transfer(TransferTask {
					source: TransferEndpoint::Device {
						device: source.device,
						value: source.value,
					},
					destination: TransferEndpoint::External,
					bytes: tensor.storage_bytes,
					route: Vec::new(),
					lane_claims: Vec::new(),
					submission,
				}),
			};
			let Some((end, makespan)) = trial_timing(topology, discovery, &state.tasks, trial)? else {
				continue;
			};
			let key = (makespan, end, source.device, source.value, source);
			if best
				.as_ref()
				.is_none_or(|known| (&key.0, &key.1, &key.2, &key.3) < (&known.0, &known.1, &known.2, &known.3))
			{
				best = Some(key);
			}
		}
		let (_, _, _, _, source) = best.ok_or_else(|| {
			let kind = if safe_count == 0 {
				PlannerErrorKind::DependencyConflict
			} else {
				PlannerErrorKind::CandidateInfeasible
			};
			PlannerError::new(
				kind,
				format!(
					"external output {} has {safe_count} non-overwritten copies but no schedulable egress",
					tensor.id
				),
			)
		})?;
		let (allocated_task, allocated_submission) = state.next_submission(source.device)?;
		if allocated_task != task || allocated_submission != submission {
			return Err(PlannerError::new(
				PlannerErrorKind::InvalidDraft,
				"stable planner ID allocation changed during egress search",
			));
		}
		state.tasks.push(UnscheduledTask {
			id: task,
			phase: RunPhase::Exit,
			dependencies: vec![source.producer],
			kind: TaskKind::Transfer(TransferTask {
				source: TransferEndpoint::Device {
					device: source.device,
					value: source.value,
				},
				destination: TransferEndpoint::External,
				bytes: tensors[&tensor.id].storage_bytes,
				route: Vec::new(),
				lane_claims: Vec::new(),
				submission,
			}),
		});
	}
	Ok(())
}

fn finalize_auxiliary_resources(
	resources: &mut ResourceManifest,
	tasks: &[Task],
	artifacts: &[ArtifactIdentity],
	artifact_builds: &[ArtifactBuildRecipe],
) -> PlannerResult<Vec<DeviceBytes>> {
	let artifact_index = artifacts
		.iter()
		.map(|artifact| (artifact.id, artifact))
		.collect::<BTreeMap<_, _>>();
	let build_index = artifact_builds
		.iter()
		.map(|build| (build.artifact, build))
		.collect::<BTreeMap<_, _>>();
	let mut staging_events = BTreeMap::<DeviceId, Vec<UsageEvent>>::new();
	let mut scratch_events = BTreeMap::<DeviceId, Vec<UsageEvent>>::new();
	for task in tasks {
		match &task.kind {
			TaskKind::Transfer(transfer) => {
				let device = match (transfer.source, transfer.destination) {
					(TransferEndpoint::External, TransferEndpoint::Device { device, .. })
					| (TransferEndpoint::Device { device, .. }, TransferEndpoint::External) => Some(device),
					(TransferEndpoint::Device { .. }, TransferEndpoint::Device { .. }) => None,
					(TransferEndpoint::External, TransferEndpoint::External) => {
						return Err(PlannerError::new(
							PlannerErrorKind::InvalidDraft,
							"external-to-external task reached staging planning",
						));
					}
				};
				if let Some(device) = device {
					push_usage(&mut staging_events, device, task.window, transfer.bytes);
				}
			}
			TaskKind::Calculation(calculation) => {
				let scratch = match (
					artifact_index.get(&calculation.artifact),
					build_index.get(&calculation.artifact),
				) {
					(Some(artifact), None) => artifact.resources.scratch_bytes_per_dispatch,
					(None, Some(build)) => build.resources.scratch_bytes_per_dispatch,
					_ => {
						return Err(PlannerError::new(
							PlannerErrorKind::InvalidArtifact,
							format!(
								"calculation task {} artifact {} does not resolve exactly once",
								task.id, calculation.artifact
							),
						));
					}
				};
				push_usage(
					&mut scratch_events,
					calculation.device,
					task.window,
					scratch,
				);
			}
			TaskKind::Metric(_) => {}
		}
	}
	let mut combined_events = staging_events.clone();
	for (device, events) in &scratch_events {
		combined_events.entry(*device).or_default().extend(events);
	}
	resources.pinned_staging = peak_usage(staging_events)?;
	resources.scratch = peak_usage(scratch_events)?;
	peak_usage(combined_events)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct UsageEvent {
	time: Nanoseconds,
	start: bool,
	bytes: ByteCount,
}

fn push_usage(
	events: &mut BTreeMap<DeviceId, Vec<UsageEvent>>,
	device: DeviceId,
	window: ScheduleWindow,
	bytes: ByteCount,
) {
	if bytes == ByteCount::ZERO {
		return;
	}
	let device_events = events.entry(device).or_default();
	device_events.push(UsageEvent {
		time: window.start,
		start: true,
		bytes,
	});
	device_events.push(UsageEvent {
		time: window.end,
		start: false,
		bytes,
	});
}

fn peak_usage(events: BTreeMap<DeviceId, Vec<UsageEvent>>) -> PlannerResult<Vec<DeviceBytes>> {
	let mut result = Vec::new();
	for (device, mut device_events) in events {
		device_events.sort();
		let mut active = ByteCount::ZERO;
		let mut peak = ByteCount::ZERO;
		for event in device_events {
			active = if event.start {
				active.checked_add(event.bytes).ok_or_else(|| {
					PlannerError::new(
						PlannerErrorKind::ArithmeticOverflow,
						format!("device {device} resource usage overflowed"),
					)
				})?
			} else {
				active.checked_sub(event.bytes).ok_or_else(|| {
					PlannerError::new(
						PlannerErrorKind::InvalidDraft,
						format!("device {device} resource usage underflowed"),
					)
				})?
			};
			peak = peak.max(active);
		}
		if peak != ByteCount::ZERO {
			result.push(DeviceBytes {
				device,
				bytes: peak,
			});
		}
	}
	Ok(result)
}

#[derive(Debug)]
struct DisjointValues {
	parent: BTreeMap<ValueId, ValueId>,
}

impl DisjointValues {
	fn new(values: &[ValueSpec]) -> Self {
		Self {
			parent: values.iter().map(|value| (value.id, value.id)).collect(),
		}
	}

	fn root(&self, mut value: ValueId) -> ValueId {
		while self.parent[&value] != value {
			value = self.parent[&value];
		}
		value
	}

	fn union(&mut self, left: ValueId, right: ValueId) {
		let left = self.root(left);
		let right = self.root(right);
		if left == right {
			return;
		}
		let (root, child) = if left < right {
			(left, right)
		} else {
			(right, left)
		};
		self.parent.insert(child, root);
	}
}

#[derive(Debug)]
struct ObjectBlueprint {
	id: ArenaObjectId,
	device: DeviceId,
	bytes: ByteCount,
	alignment: ByteCount,
	members: Vec<ValueId>,
}

fn build_arena_contract(
	values: &[ValueSpec],
	tasks: &[Task],
	images: &[ImageRegion],
	aliases: &[AliasInvocation],
) -> PlannerResult<(Vec<ArenaObject>, Vec<ValueBinding>)> {
	let specs = values
		.iter()
		.map(|value| (value.id, value))
		.collect::<BTreeMap<_, _>>();
	let mut disjoint = DisjointValues::new(values);
	for invocation in aliases {
		for rule in &invocation.rules {
			if rule.permission != AliasPermission::MustAliasExact {
				continue;
			}
			let input_index = usize::try_from(rule.input.get().saturating_sub(1)).map_err(|_| {
				PlannerError::new(
					PlannerErrorKind::ArithmeticOverflow,
					"kernel input index does not fit usize",
				)
			})?;
			let output_index = usize::try_from(rule.output.get().saturating_sub(1)).map_err(|_| {
				PlannerError::new(
					PlannerErrorKind::ArithmeticOverflow,
					"kernel output index does not fit usize",
				)
			})?;
			let input = invocation.inputs.get(input_index).copied().ok_or_else(|| {
				PlannerError::new(
					PlannerErrorKind::InvalidDraft,
					"must-alias input index is absent from invocation",
				)
			})?;
			let output = invocation
				.outputs
				.get(output_index)
				.copied()
				.ok_or_else(|| {
					PlannerError::new(
						PlannerErrorKind::InvalidDraft,
						"must-alias output index is absent from invocation",
					)
				})?;
			disjoint.union(input, output);
		}
	}
	let mut groups = BTreeMap::<ValueId, Vec<ValueId>>::new();
	for value in values {
		groups.entry(disjoint.root(value.id))
			.or_default()
			.push(value.id);
	}
	let mut object_ids = StableIdAllocator::new("arena object");
	let mut blueprints = Vec::new();
	let mut fixed = BTreeMap::<ValueId, (ArenaObjectId, ByteOffset)>::new();
	for image in images {
		let object = ArenaObjectId::new(object_ids.take()?);
		let mut members = Vec::new();
		for (value, offset) in &image.members {
			fixed.insert(*value, (object, *offset));
			members.push(*value);
		}
		blueprints.push(ObjectBlueprint {
			id: object,
			device: image.device,
			bytes: image.bytes,
			alignment: ByteCount::new(16),
			members,
		});
	}

	let mut bindings = BTreeMap::<ValueId, ValueBinding>::new();
	for members in groups.values() {
		let first = specs[&members[0]];
		for member in members {
			let spec = specs[member];
			if spec.device != first.device || spec.bytes != first.bytes {
				return Err(PlannerError::new(
					PlannerErrorKind::InvalidDraft,
					"must-alias values differ in device or exact byte size",
				));
			}
		}
		let locations = members
			.iter()
			.filter_map(|member| fixed.get(member).copied())
			.collect::<BTreeSet<_>>();
		let (object, offset) = match locations.len() {
			0 => {
				let object = ArenaObjectId::new(object_ids.take()?);
				blueprints.push(ObjectBlueprint {
					id: object,
					device: first.device,
					bytes: first.bytes,
					alignment: ByteCount::new(16),
					members: members.clone(),
				});
				(object, ByteOffset::new(0))
			}
			1 => locations.iter().next().copied().ok_or_else(|| {
				PlannerError::new(
					PlannerErrorKind::InvalidDraft,
					"fixed arena location disappeared",
				)
			})?,
			_ => {
				return Err(PlannerError::new(
					PlannerErrorKind::InvalidDraft,
					"must-alias values occupy different data-image locations",
				));
			}
		};
		for member in members {
			if !blueprints
				.iter_mut()
				.find(|blueprint| blueprint.id == object)
				.is_some_and(|blueprint| {
					if !blueprint.members.contains(member) {
						blueprint.members.push(*member);
					}
					true
				}) {
				return Err(PlannerError::new(
					PlannerErrorKind::InvalidDraft,
					format!("arena object {object} is absent"),
				));
			}
			bindings.insert(
				*member,
				ValueBinding {
					value: *member,
					object,
					object_offset: offset,
				},
			);
		}
	}

	let task_index = tasks
		.iter()
		.map(|task| (task.id, task))
		.collect::<BTreeMap<_, _>>();
	let mut objects = Vec::with_capacity(blueprints.len());
	for mut blueprint in blueprints {
		blueprint.members.sort();
		blueprint.members.dedup();
		let mut start = None;
		let mut end = None;
		for value in &blueprint.members {
			let spec = specs[value];
			if let Some(producer) = spec.producer {
				let window = task_index
					.get(&producer)
					.ok_or_else(|| {
						PlannerError::new(
							PlannerErrorKind::InvalidDraft,
							format!("value {value} producer task {producer} is absent"),
						)
					})?
					.window;
				extend_lifetime(&mut start, &mut end, window);
			}
			for task in tasks.iter().filter(|task| task_references(task, *value)) {
				extend_lifetime(&mut start, &mut end, task.window);
			}
		}
		let lifetime = ScheduleWindow {
			start: start.ok_or_else(|| {
				PlannerError::new(
					PlannerErrorKind::InvalidDraft,
					format!("arena object {} has no live task", blueprint.id),
				)
			})?,
			end: end.ok_or_else(|| {
				PlannerError::new(
					PlannerErrorKind::InvalidDraft,
					format!("arena object {} has no lifetime end", blueprint.id),
				)
			})?,
		};
		objects.push(ArenaObject {
			id: blueprint.id,
			device: blueprint.device,
			bytes: blueprint.bytes,
			alignment: blueprint.alignment,
			lifetime,
		});
	}
	objects.sort_by_key(|object| object.id);
	Ok((objects, bindings.into_values().collect()))
}

fn extend_lifetime(start: &mut Option<Nanoseconds>, end: &mut Option<Nanoseconds>, window: ScheduleWindow) {
	*start = Some(start.map_or(window.start, |known| known.min(window.start)));
	*end = Some(end.map_or(window.end, |known| known.max(window.end)));
}

fn task_references(task: &Task, value: ValueId) -> bool {
	task_kind_references(&task.kind, value)
}

fn task_kind_references(kind: &TaskKind, value: ValueId) -> bool {
	match kind {
		TaskKind::Calculation(calculation) => {
			calculation.inputs.contains(&value)
				|| calculation.outputs.contains(&value)
				|| calculation.fault_flag == Some(value)
		}
		TaskKind::Transfer(transfer) => {
			matches!(
				transfer.source,
				TransferEndpoint::Device {
					value: candidate,
					..
				} if candidate == value
			) || matches!(
				transfer.destination,
				TransferEndpoint::Device {
					value: candidate,
					..
				} if candidate == value
			)
		}
		TaskKind::Metric(metric) => metric.value == value,
	}
}

fn require_total_capacity(
	capacity: &CapacityLedger,
	layouts: &[ArenaLayout],
	auxiliary_peak: &[DeviceBytes],
) -> PlannerResult<()> {
	for layout in layouts {
		let auxiliary = auxiliary_peak
			.iter()
			.find(|entry| entry.device == layout.device)
			.map_or(ByteCount::ZERO, |entry| entry.bytes);
		let required = layout.size.checked_add(auxiliary).ok_or_else(|| {
			PlannerError::new(
				PlannerErrorKind::CandidateInfeasible,
				format!("device {} total planned bytes overflowed", layout.device),
			)
		})?;
		let usable = capacity.entry(layout.device).ok_or_else(|| {
			PlannerError::new(
				PlannerErrorKind::InvalidCapacity,
				format!("device {} has no capacity entry", layout.device),
			)
		})?;
		if required > usable.recipe_usable.value {
			return Err(PlannerError::new(
				PlannerErrorKind::InvalidCapacity,
				format!(
					"device {} needs {} arena/staging/scratch bytes but only {} are usable",
					layout.device,
					required.get(),
					usable.recipe_usable.value.get()
				),
			));
		}
	}
	Ok(())
}

#[derive(Clone, Copy, Debug)]
struct DraftHashInput<'a> {
	candidate: CandidateIdentity,
	topology: &'a Topology,
	discovery: &'a DiscoveryProfile,
	values: &'a [ValueSpec],
	kernels: &'a [KernelTemplate],
	tasks: &'a [Task],
	artifacts: &'a [ArtifactIdentity],
	artifact_builds: &'a [ArtifactBuildRecipe],
	resources: &'a ResourceManifest,
	arena_objects: &'a [ArenaObject],
	value_bindings: &'a [ValueBinding],
	value_aliases: &'a [ValueAliasContract],
	init_images: &'a [InitDataImage],
	releases: &'a [ArenaRelease],
}

fn hash_draft(input: &DraftHashInput<'_>) -> DraftIdentity {
	let mut digest = StableDigest::new("recipe-planner-draft-v7");
	digest.digest(input.candidate.digest());
	digest.digest(input.topology.identity.digest());
	digest.digest(input.discovery.identity.digest());
	digest.length(input.values.len());
	for value in input.values {
		digest.u64(value.id.get());
		digest.u8(dtype_tag(value.dtype));
		digest.u64(value.bytes.get());
		digest.u64(value.device.get());
		match value.producer {
			Some(producer) => {
				digest.u8(1);
				digest.u64(producer.get());
			}
			None => digest.u8(0),
		}
	}
	digest.length(input.kernels.len());
	for kernel in input.kernels {
		digest.u64(kernel.id.get());
		hash_kernel_template(&mut digest, kernel);
	}
	digest.length(input.tasks.len());
	for task in input.tasks {
		digest.u64(task.id.get());
		digest.u8(match task.phase {
			RunPhase::Init => 0,
			RunPhase::Loop => 1,
			RunPhase::Exit => 2,
		});
		digest.u64(task.window.start.get());
		digest.u64(task.window.end.get());
		digest.length(task.dependencies.len());
		for dependency in &task.dependencies {
			digest.u64(dependency.get());
		}
		match &task.kind {
			TaskKind::Calculation(calculation) => {
				digest.u8(0);
				digest.u64(calculation.device.get());
				digest.u64(calculation.kernel_template.get());
				digest.u64(calculation.artifact.get());
				digest.u64(calculation.work.get());
				digest.length(calculation.inputs.len());
				for value in &calculation.inputs {
					digest.u64(value.get());
				}
				digest.length(calculation.outputs.len());
				for value in &calculation.outputs {
					digest.u64(value.get());
				}
				match calculation.fault_flag {
					Some(fault_flag) => {
						digest.u8(1);
						digest.u64(fault_flag.get());
					}
					None => digest.u8(0),
				}
				digest.u64(calculation.submission.queue.get());
				digest.u64(calculation.submission.completion.get());
			}
			TaskKind::Transfer(transfer) => {
				digest.u8(1);
				digest.u64(transfer.bytes.get());
				hash_endpoint(&mut digest, transfer.source);
				hash_endpoint(&mut digest, transfer.destination);
				digest.length(transfer.route.len());
				for link in &transfer.route {
					digest.u64(link.get());
				}
				digest.length(transfer.lane_claims.len());
				for claim in &transfer.lane_claims {
					match claim {
						TransferLaneClaim::Link { link, lane } => {
							digest.u8(0);
							digest.u64(link.get());
							digest.u64(u64::from(*lane));
						}
						TransferLaneClaim::External { device, lane } => {
							digest.u8(1);
							digest.u64(device.get());
							digest.u64(u64::from(*lane));
						}
					}
				}
				digest.u64(transfer.submission.queue.get());
				digest.u64(transfer.submission.completion.get());
			}
			TaskKind::Metric(metric) => {
				digest.u8(2);
				match metric.purpose {
					MetricPurpose::User => digest.u8(0),
					MetricPurpose::FaultReadback { calculation } => {
						digest.u8(1);
						digest.u64(calculation.get());
					}
				}
				digest.u64(metric.metric.get());
				digest.u64(metric.value.get());
				digest.u64(metric.slot.get());
			}
		}
	}
	digest.length(input.artifacts.len());
	for artifact in input.artifacts {
		hash_artifact(&mut digest, artifact);
	}
	digest.length(input.artifact_builds.len());
	for build in input.artifact_builds {
		hash_artifact_build(&mut digest, build);
	}
	digest.length(input.resources.queues.len());
	for queue in &input.resources.queues {
		digest.u64(queue.id.get());
		digest.u64(queue.device.get());
	}
	digest.length(input.resources.completions.len());
	for completion in &input.resources.completions {
		digest.u64(completion.id.get());
		digest.u64(completion.device.get());
	}
	digest.length(input.resources.metrics.len());
	for metric in &input.resources.metrics {
		digest.u64(metric.id.get());
		digest.u64(metric.metric.get());
	}
	hash_device_bytes(&mut digest, &input.resources.pinned_staging);
	hash_device_bytes(&mut digest, &input.resources.scratch);
	digest.length(input.arena_objects.len());
	for object in input.arena_objects {
		digest.u64(object.id.get());
		digest.u64(object.device.get());
		digest.u64(object.bytes.get());
		digest.u64(object.alignment.get());
		digest.u64(object.lifetime.start.get());
		digest.u64(object.lifetime.end.get());
	}
	digest.length(input.value_bindings.len());
	for binding in input.value_bindings {
		digest.u64(binding.value.get());
		digest.u64(binding.object.get());
		digest.u64(binding.object_offset.get());
	}
	digest.length(input.value_aliases.len());
	for alias in input.value_aliases {
		digest.u64(alias.input.get());
		digest.u64(alias.output.get());
		digest.u8(alias_permission_tag(alias.permission));
	}
	digest.length(input.init_images.len());
	for image in input.init_images {
		digest.u64(image.device.get());
		digest.u64(image.image.get());
		digest.u64(image.bytes.get());
		digest.length(image.members.len());
		for member in &image.members {
			digest.u64(member.logical.get());
			digest.u64(member.physical.get());
			digest.u8(dtype_tag(member.dtype));
			digest.u64(member.bytes.get());
			digest.u64(member.image_offset.get());
		}
	}
	digest.length(input.releases.len());
	for release in input.releases {
		digest.u64(release.device.get());
	}
	DraftIdentity::new(digest.finish())
}

fn hash_artifact_build(digest: &mut StableDigest, build: &ArtifactBuildRecipe) {
	digest.u64(build.artifact.get());
	digest.u64(build.kernel_template.get());
	digest.u64(build.source_kernel.get());
	digest.digest(build.provenance.program_digest);
	digest.u64(u64::from(build.provenance.stage_ordinal));
	digest.digest(build.provenance.contract_digest);
	digest.length(build.bindings.len());
	for binding in &build.bindings {
		digest.u64(binding.value.get());
		digest.u8(dtype_tag(binding.dtype));
		digest.u8(match binding.access {
			ArtifactBuildAccess::Read => 0,
			ArtifactBuildAccess::Write => 1,
			ArtifactBuildAccess::ReadWrite => 2,
			ArtifactBuildAccess::ReadWriteAtomic => 3,
		});
		digest.length(binding.view.logical_extents.len());
		for extent in &binding.view.logical_extents {
			digest.u64(*extent);
		}
		digest.u64(binding.view.offset_elements);
		digest.length(binding.view.strides.len());
		for stride in &binding.view.strides {
			digest.u64(*stride);
		}
		digest.u64(binding.view.storage_bytes.get());
	}
	digest.u64(build.dispatch.logical_lanes);
	digest.u64(u64::from(build.dispatch.workgroup_lanes));
	digest.u64(build.dispatch.workgroups);
	digest.u64(build.work.flops.get());
	digest.u64(build.work.integer_operations);
	digest.u64(build.work.atomic_operations);
	match build.fault_flag {
		Some(fault_flag) => {
			digest.u8(1);
			digest.u64(fault_flag.get());
		}
		None => digest.u8(0),
	}
	digest.u64(build.resources.private_bytes_per_lane.get());
	digest.u64(build.resources.shared_bytes_per_workgroup.get());
	digest.u64(build.resources.scratch_bytes_per_dispatch.get());
	digest.u64(u64::from(build.resources.maximum_workgroup_lanes));
}

fn hash_device_bytes(digest: &mut StableDigest, values: &[DeviceBytes]) {
	digest.length(values.len());
	for value in values {
		digest.u64(value.device.get());
		digest.u64(value.bytes.get());
	}
}

fn hash_endpoint(digest: &mut StableDigest, endpoint: TransferEndpoint) {
	match endpoint {
		TransferEndpoint::External => digest.u8(0),
		TransferEndpoint::Device { device, value } => {
			digest.u8(1);
			digest.u64(device.get());
			digest.u64(value.get());
		}
	}
}
