use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::{
	artifact::{ArtifactBuildRecipe, ArtifactIdentity},
	discovery::DiscoveryProfile,
	error::{ValidationCode, ValidationResult, Validator},
	identity::{
		BundleIdentity, CandidateIdentity, DiscoveryIdentity, DraftIdentity, Label, RealizationIdentity,
		TopologyIdentity,
	},
	ids::{ArenaObjectId, ArtifactId, DeviceId, TaskId, ValueId},
	scalar::{AliasPermission, DType, KernelTemplate},
	schedule::{
		ArenaLayout, ArenaObject, ArenaRelease, InitDataImage, IterationDomain, LoopIterations, LoopSchedule,
		LoopTaskDomain, MetricPurpose, ResolvedTransferEndpoint, ResolvedTransferEndpoints, ResolvedValueLocation,
		ResourceManifest, RunPhase, Task, TaskKind, TransferEndpoint, TransferLaneClaim, ValueAliasContract,
		ValueBinding, ValueSpec,
	},
	topology::{DeviceKind, Property, Topology},
	units::{ByteCount, Nanoseconds},
};

pub const EXACT_USER_RESERVATION: ByteCount = ByteCount::new(1_000_000_000);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReservationMechanism {
	HeldAllocation,
	EnforcedQuota,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReservationEvidence {
	NonGpu,
	GpuDisplay { enabled_connectors: u32 },
}

impl ReservationEvidence {
	#[must_use]
	pub const fn required_bytes(self) -> ByteCount {
		match self {
			Self::NonGpu
			| Self::GpuDisplay {
				enabled_connectors: 1..=u32::MAX,
			} => EXACT_USER_RESERVATION,
			Self::GpuDisplay {
				enabled_connectors: 0,
			} => ByteCount::ZERO,
		}
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReservationEntry {
	pub device: DeviceId,
	pub name: Label,
	pub bytes: ByteCount,
	pub mechanism: ReservationMechanism,
	pub evidence: ReservationEvidence,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReservationLedger {
	pub entries: Vec<ReservationEntry>,
}

impl ReservationLedger {
	pub fn validate(&self, topology: &Topology) -> ValidationResult {
		let mut validator = Validator::default();
		let mut seen = BTreeSet::new();
		for (index, entry) in self.entries.iter().enumerate() {
			let device = topology.device(entry.device);
			validator.require(
				device.is_some(),
				ValidationCode::UnknownReference,
				format!("entries[{index}].device"),
				format!("unknown device {}", entry.device),
			);
			validator.require(
				seen.insert(entry.device),
				ValidationCode::DuplicateReservation,
				format!("entries[{index}].device"),
				format!("device {} has more than one user reservation", entry.device),
			);
			if let Some(device) = device {
				let evidence_matches = matches!(
					(device.kind, entry.evidence),
					(
						DeviceKind::Ram | DeviceKind::Disk,
						ReservationEvidence::NonGpu
					) | (
						DeviceKind::GpuMemory,
						ReservationEvidence::GpuDisplay { .. }
					)
				);
				validator.require(
					evidence_matches,
					ValidationCode::WrongKind,
					format!("entries[{index}].evidence"),
					format!(
						"reservation evidence {:?} does not match device kind {:?}",
						entry.evidence, device.kind
					),
				);
			}
			let required_bytes = entry.evidence.required_bytes();
			validator.require(
				entry.bytes == required_bytes,
				ValidationCode::WrongReservationSize,
				format!("entries[{index}].bytes"),
				format!(
					"reservation evidence requires exactly {} bytes, got {}",
					required_bytes.get(),
					entry.bytes.get()
				),
			);
		}
		for (index, device) in topology.devices.iter().enumerate() {
			validator.require(
				seen.contains(&device.id),
				ValidationCode::MissingReservation,
				format!("topology.devices[{index}]"),
				format!("required device {} has no user reservation", device.id),
			);
		}
		validator.finish()
	}

	#[must_use]
	pub fn entry(&self, device: DeviceId) -> Option<&ReservationEntry> {
		self.entries.iter().find(|entry| entry.device == device)
	}
}

/// Post-stabilization capacity accounting for one required device.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapacityLedgerEntry {
	pub device: DeviceId,
	pub total: Property<ByteCount>,
	pub runtime_overhead: Property<ByteCount>,
	pub fragmentation: Property<ByteCount>,
	pub safety_headroom: Property<ByteCount>,
	pub recipe_usable: Property<ByteCount>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapacityLedger {
	pub entries: Vec<CapacityLedgerEntry>,
}

impl CapacityLedger {
	pub fn validate(&self, topology: &Topology, reservations: &ReservationLedger) -> ValidationResult {
		let mut validator = Validator::default();
		let mut seen = BTreeSet::new();
		for (index, entry) in self.entries.iter().enumerate() {
			validator.require(
				topology.device(entry.device).is_some(),
				ValidationCode::UnknownReference,
				format!("entries[{index}].device"),
				format!("unknown device {}", entry.device),
			);
			validator.require(
				seen.insert(entry.device),
				ValidationCode::DuplicateId,
				format!("entries[{index}].device"),
				format!("device {} has more than one capacity entry", entry.device),
			);
			let Some(reservation) = reservations.entry(entry.device) else {
				continue;
			};
			for (field, schedulable) in [
				("total", entry.total.is_schedulable()),
				("runtime_overhead", entry.runtime_overhead.is_schedulable()),
				("fragmentation", entry.fragmentation.is_schedulable()),
				("safety_headroom", entry.safety_headroom.is_schedulable()),
				("recipe_usable", entry.recipe_usable.is_schedulable()),
			] {
				validator.require(
					schedulable,
					ValidationCode::UnmeasuredProperty,
					format!("entries[{index}].{field}"),
					"realization capacity must be measured or explicitly overridden",
				);
			}
			let accounted = [
				reservation.bytes,
				entry.runtime_overhead.value,
				entry.fragmentation.value,
				entry.safety_headroom.value,
				entry.recipe_usable.value,
			]
			.into_iter()
			.try_fold(ByteCount::ZERO, ByteCount::checked_add);
			match accounted {
				Some(accounted) => {
					validator.require(
						accounted <= entry.total.value,
						ValidationCode::CapacityOverflow,
						format!("entries[{index}]"),
						format!(
							"accounted capacity {} exceeds total {}",
							accounted.get(),
							entry.total.value.get()
						),
					)
				}
				None => {
					validator.error(
						ValidationCode::CapacityOverflow,
						format!("entries[{index}]"),
						"capacity accounting overflowed",
					)
				}
			}
		}
		for (index, device) in topology.devices.iter().enumerate() {
			validator.require(
				seen.contains(&device.id),
				ValidationCode::MissingRequiredObject,
				format!("topology.devices[{index}]"),
				format!("required device {} has no capacity entry", device.id),
			);
		}
		validator.finish()
	}

	#[must_use]
	pub fn entry(&self, device: DeviceId) -> Option<&CapacityLedgerEntry> {
		self.entries.iter().find(|entry| entry.device == device)
	}
}

/// Exact, offset-free execution candidate emitted by Draft.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DraftPlan {
	pub identity: DraftIdentity,
	pub candidate: CandidateIdentity,
	pub discovery: DiscoveryIdentity,
	pub topology: TopologyIdentity,
	pub values: Vec<ValueSpec>,
	pub kernels: Vec<KernelTemplate>,
	pub artifacts: Vec<ArtifactIdentity>,
	pub artifact_builds: Vec<ArtifactBuildRecipe>,
	pub tasks: Vec<Task>,
	pub resources: ResourceManifest,
	pub arena_objects: Vec<ArenaObject>,
	pub value_bindings: Vec<ValueBinding>,
	pub value_aliases: Vec<ValueAliasContract>,
	pub init_images: Vec<InitDataImage>,
	pub releases: Vec<ArenaRelease>,
}

impl DraftPlan {
	pub fn validate(&self, topology: &Topology, discovery: &DiscoveryProfile) -> ValidationResult {
		let mut validator = Validator::default();
		if let Err(errors) = topology.validate() {
			validator.append("topology", errors);
		}
		if let Err(errors) = topology.validate_scheduling_properties() {
			validator.append("topology.scheduling", errors);
		}
		if let Err(errors) = discovery.validate(topology) {
			validator.append("discovery", errors);
		}
		for (path, valid, message) in [
			(
				"identity",
				!self.identity.is_zero(),
				"draft identity must not be zero",
			),
			(
				"candidate",
				!self.candidate.is_zero(),
				"candidate identity must not be zero",
			),
			(
				"discovery",
				self.discovery == discovery.identity,
				"draft belongs to a different discovery profile",
			),
			(
				"topology",
				self.topology == topology.identity,
				"draft belongs to a different topology",
			),
		] {
			validator.require(
				valid,
				if path == "identity" || path == "candidate" {
					ValidationCode::InvalidIdentity
				} else {
					ValidationCode::IdentityMismatch
				},
				path,
				message,
			);
		}
		if let Err(errors) = self.resources.validate(topology) {
			validator.append("resources", errors);
		}

		let mut values = BTreeMap::new();
		for (index, value) in self.values.iter().enumerate() {
			validator.require(
				values.insert(value.id, value).is_none(),
				ValidationCode::DuplicateId,
				format!("values[{index}].id"),
				format!("value {} appears more than once", value.id),
			);
			validator.require(
				topology.device(value.device).is_some(),
				ValidationCode::UnknownReference,
				format!("values[{index}].device"),
				format!("unknown device {}", value.device),
			);
			validator.require(
				value.bytes.get() % u64::from(value.dtype.byte_width()) == 0,
				ValidationCode::WrongKind,
				format!("values[{index}].bytes"),
				"payload storage size must be a whole number of typed elements",
			);
		}

		let mut kernels = BTreeMap::new();
		for (index, kernel) in self.kernels.iter().enumerate() {
			validator.require(
				kernels.insert(kernel.id, kernel).is_none(),
				ValidationCode::DuplicateId,
				format!("kernels[{index}].id"),
				format!("kernel template {} appears more than once", kernel.id),
			);
			if let Err(errors) = kernel.validate() {
				validator.append(&format!("kernels[{index}]"), errors);
			}
		}

		let mut artifacts = BTreeMap::new();
		for (index, artifact) in self.artifacts.iter().enumerate() {
			validator.require(
				artifacts.insert(artifact.id, artifact).is_none(),
				ValidationCode::DuplicateId,
				format!("artifacts[{index}].id"),
				format!("artifact {} appears more than once", artifact.id),
			);
			if let Err(errors) = artifact.validate() {
				validator.append(&format!("artifacts[{index}]"), errors);
			}
		}

		let mut artifact_builds = BTreeMap::new();
		let mut stage_templates = BTreeMap::new();
		for (index, build) in self.artifact_builds.iter().enumerate() {
			validator.require(
				artifact_builds.insert(build.artifact, build).is_none(),
				ValidationCode::DuplicateId,
				format!("artifact_builds[{index}].artifact"),
				format!(
					"deferred artifact {} appears more than once",
					build.artifact
				),
			);
			validator.require(
				!artifacts.contains_key(&build.artifact),
				ValidationCode::DuplicateId,
				format!("artifact_builds[{index}].artifact"),
				format!(
					"artifact {} is both realized and deferred in one Draft",
					build.artifact
				),
			);
			validator.require(
				stage_templates
					.insert(build.kernel_template, build.artifact)
					.is_none(),
				ValidationCode::DuplicateId,
				format!("artifact_builds[{index}].kernel_template"),
				format!(
					"stage-scoped kernel template {} appears more than once",
					build.kernel_template
				),
			);
			if let Err(errors) = build.validate() {
				validator.append(&format!("artifact_builds[{index}]"), errors);
			}
			for (binding_index, binding) in build.bindings.iter().enumerate() {
				validator.require(
					values.contains_key(&binding.value),
					ValidationCode::UnknownReference,
					format!("artifact_builds[{index}].bindings[{binding_index}].value"),
					format!("unknown value {}", binding.value),
				);
			}
		}

		validate_tasks(&mut validator, self, topology, discovery, &DraftIndexes {
			values: &values,
			kernels: &kernels,
			artifacts: &artifacts,
			artifact_builds: &artifact_builds,
		});
		validate_arena_objects(&mut validator, self, topology);
		validate_value_bindings(&mut validator, self, &values, &kernels);
		validate_init_images(&mut validator, self, topology, &values);
		validate_releases(&mut validator, self, topology);
		validator.finish()
	}
}

#[derive(Clone, Copy, Debug)]
struct DraftIndexes<'a> {
	values: &'a BTreeMap<ValueId, &'a ValueSpec>,
	kernels: &'a BTreeMap<crate::ids::KernelTemplateId, &'a KernelTemplate>,
	artifacts: &'a BTreeMap<ArtifactId, &'a ArtifactIdentity>,
	artifact_builds: &'a BTreeMap<ArtifactId, &'a ArtifactBuildRecipe>,
}

fn validate_tasks(
	validator: &mut Validator,
	draft: &DraftPlan,
	topology: &Topology,
	discovery: &DiscoveryProfile,
	indexes: &DraftIndexes<'_>,
) {
	let mut tasks = BTreeMap::<TaskId, &Task>::new();
	for (index, task) in draft.tasks.iter().enumerate() {
		validator.require(
			tasks.insert(task.id, task).is_none(),
			ValidationCode::DuplicateId,
			format!("tasks[{index}].id"),
			format!("task {} appears more than once", task.id),
		);
		validator.require(
			task.window.is_valid(),
			ValidationCode::InvalidLifetime,
			format!("tasks[{index}].window"),
			"task schedule window must be nonempty",
		);
	}

	let mut upload_counts = BTreeMap::<DeviceId, usize>::new();
	for (index, task) in draft.tasks.iter().enumerate() {
		let path = format!("tasks[{index}]");
		let mut deps = BTreeSet::new();
		for (dep_index, dependency) in task.dependencies.iter().copied().enumerate() {
			validator.require(
				deps.insert(dependency),
				ValidationCode::DuplicateId,
				format!("{path}.dependencies[{dep_index}]"),
				format!("dependency {dependency} appears more than once"),
			);
			let Some(predecessor) = tasks.get(&dependency).copied() else {
				validator.error(
					ValidationCode::UnknownReference,
					format!("{path}.dependencies[{dep_index}]"),
					format!("unknown task {dependency}"),
				);
				continue;
			};
			validator.require(
				predecessor.phase <= task.phase,
				ValidationCode::DependencyPhaseOrder,
				format!("{path}.dependencies[{dep_index}]"),
				"task depends on a later lifecycle phase",
			);
			validator.require(
				predecessor.window.end <= task.window.start,
				ValidationCode::DependencyScheduleOrder,
				format!("{path}.dependencies[{dep_index}]"),
				"dependency does not complete before this task starts",
			);
		}

		match &task.kind {
			TaskKind::Calculation(calculation) => {
				validator.require(
					task.phase == RunPhase::Loop,
					ValidationCode::InvalidPhase,
					format!("{path}.phase"),
					"calculation tasks belong to the loop",
				);
				let device = topology.device(calculation.device);
				validator.require(
					device.is_some_and(|device| device.kind == DeviceKind::GpuMemory),
					ValidationCode::InvalidCalculationPlacement,
					format!("{path}.device"),
					"calculation must be placed on a GPU storage device",
				);
				let kernel = indexes.kernels.get(&calculation.kernel_template).copied();
				let artifact = indexes.artifacts.get(&calculation.artifact).copied();
				let build = indexes.artifact_builds.get(&calculation.artifact).copied();
				validator.require(
					artifact.is_some() != build.is_some(),
					ValidationCode::UnknownReference,
					format!("{path}.artifact"),
					format!(
						"artifact {} must resolve to exactly one realized identity or deferred build",
						calculation.artifact
					),
				);
				if let Some(artifact) = artifact {
					validator.require(
						kernel.is_some(),
						ValidationCode::UnknownReference,
						format!("{path}.kernel_template"),
						format!("unknown kernel template {}", calculation.kernel_template),
					);
					validator.require(
						artifact.kernel_template == calculation.kernel_template,
						ValidationCode::ArtifactMismatch,
						format!("{path}.artifact"),
						"artifact and calculation stage identities differ",
					);
					if let Some(discovered) = discovery.device(calculation.device) {
						validator.require(
							discovered
								.calculation
								.as_ref()
								.is_some_and(|capability| capability.target == artifact.target),
							ValidationCode::ArtifactMismatch,
							format!("{path}.artifact"),
							"artifact target does not match the discovered calculation target",
						);
					}
				}
				if let Some(build) = build {
					validator.require(
						build.kernel_template == calculation.kernel_template,
						ValidationCode::ArtifactMismatch,
						format!("{path}.kernel_template"),
						"calculation does not name its deferred stage identity",
					);
					validator.require(
						build.work.flops == calculation.work,
						ValidationCode::ResourceMismatch,
						format!("{path}.work"),
						"calculation FLOP work differs from its deferred stage",
					);
					let expected_inputs = build
						.bindings
						.iter()
						.filter(|binding| build.fault_flag != Some(binding.value) && binding.access.reads())
						.map(|binding| binding.value)
						.collect::<Vec<_>>();
					let expected_outputs = build
						.bindings
						.iter()
						.filter(|binding| {
							build.fault_flag != Some(binding.value) && binding.access.writes()
						})
						.map(|binding| binding.value)
						.collect::<Vec<_>>();
					validator.require(
						calculation.inputs == expected_inputs,
						ValidationCode::ResourceMismatch,
						format!("{path}.inputs"),
						"calculation inputs differ from the ordered deferred bindings",
					);
					validator.require(
						calculation.outputs == expected_outputs,
						ValidationCode::ResourceMismatch,
						format!("{path}.outputs"),
						"calculation outputs differ from the ordered deferred bindings",
					);
					for (binding_index, binding) in build.bindings.iter().enumerate() {
						if let Some(value) = indexes.values.get(&binding.value).copied() {
							validator.require(
								value.device == calculation.device
									&& value.dtype == binding.dtype && value.bytes
									== binding.view.storage_bytes,
								ValidationCode::ResourceMismatch,
								format!("{path}.artifact_build.bindings[{binding_index}]"),
								"deferred binding differs from its resident value",
							);
						}
					}
				}
				for (field, ids) in [
					("inputs", &calculation.inputs),
					("outputs", &calculation.outputs),
				] {
					for (value_index, value_id) in ids.iter().enumerate() {
						let value = indexes.values.get(value_id).copied();
						validator.require(
							value.is_some(),
							ValidationCode::UnknownReference,
							format!("{path}.{field}[{value_index}]"),
							format!("unknown value {value_id}"),
						);
						if let Some(value) = value {
							validator.require(
								value.device == calculation.device,
								ValidationCode::InvalidCalculationPlacement,
								format!("{path}.{field}[{value_index}]"),
								"calculation value is not resident on its GPU device",
							);
						}
					}
				}
				let requires_fault_flag = match build {
					Some(build) => build.fault_flag.is_some(),
					None => kernel.is_some_and(|kernel| kernel.program.requires_fault_flag()),
				};
				validator.require(
					calculation.fault_flag.is_some() == requires_fault_flag,
					ValidationCode::ResourceMismatch,
					format!("{path}.fault_flag"),
					"calculation fault flag presence differs from its kernel contract",
				);
				if let Some(fault_flag) = calculation.fault_flag {
					let value = indexes.values.get(&fault_flag).copied();
					validator.require(
						value.is_some(),
						ValidationCode::UnknownReference,
						format!("{path}.fault_flag"),
						format!("unknown fault flag value {fault_flag}"),
					);
					if let Some(value) = value {
						validator.require(
							value.device == calculation.device
								&& value.dtype == DType::I32 && value.bytes == ByteCount::new(4),
							ValidationCode::ResourceMismatch,
							format!("{path}.fault_flag"),
							"fault flag must be one int32 resident on the calculation GPU",
						);
					}
				}
				validate_submission(
					validator,
					&path,
					calculation.device,
					calculation.submission,
					&draft.resources,
				);
			}
			TaskKind::Transfer(transfer) => {
				let source_value = validate_transfer_endpoint(
					validator,
					&path,
					"source",
					transfer.source,
					transfer.bytes,
					indexes.values,
				);
				let destination_value = validate_transfer_endpoint(
					validator,
					&path,
					"destination",
					transfer.destination,
					transfer.bytes,
					indexes.values,
				);
				match (transfer.source, transfer.destination) {
					(
						TransferEndpoint::External,
						TransferEndpoint::Device {
							device: destination,
							..
						},
					) => {
						validator.require(
							task.phase == RunPhase::Init,
							ValidationCode::InvalidExternalTransfer,
							format!("{path}.phase"),
							"external admission is legal only during init",
						);
						validator.require(
							transfer.route.is_empty(),
							ValidationCode::InvalidRoute,
							format!("{path}.route"),
							"logical external admission has no internal route",
						);
						*upload_counts.entry(destination).or_default() += 1;
						validator.require(
							destination_value.is_some(),
							ValidationCode::ResourceMismatch,
							format!("{path}.destination"),
							"init value is not bound to the destination device",
						);
						validate_submission(
							validator,
							&path,
							destination,
							transfer.submission,
							&draft.resources,
						);
					}
					(TransferEndpoint::Device { device: source, .. }, TransferEndpoint::External) => {
						validator.require(
							task.phase == RunPhase::Exit,
							ValidationCode::InvalidExternalTransfer,
							format!("{path}.phase"),
							"external egress is legal only during exit",
						);
						validator.require(
							transfer.route.is_empty(),
							ValidationCode::InvalidRoute,
							format!("{path}.route"),
							"logical external egress has no internal route",
						);
						validator.require(
							source_value.is_some(),
							ValidationCode::ResourceMismatch,
							format!("{path}.source"),
							"exit value is not bound to the source device",
						);
						validate_submission(
							validator,
							&path,
							source,
							transfer.submission,
							&draft.resources,
						);
					}
					(
						TransferEndpoint::Device { device: source, .. },
						TransferEndpoint::Device {
							device: destination,
							..
						},
					) => {
						validator.require(
							source_value.is_some(),
							ValidationCode::ResourceMismatch,
							format!("{path}.source"),
							"internal-transfer value is not bound to the source device",
						);
						validator.require(
							destination_value.is_some(),
							ValidationCode::ResourceMismatch,
							format!("{path}.destination"),
							"internal-transfer value is not bound to the destination device",
						);
						validator.require(
							source_value
								.zip(destination_value)
								.is_some_and(|(source, destination)| source.dtype == destination.dtype),
							ValidationCode::ResourceMismatch,
							format!("{path}.destination"),
							"internal-transfer source and destination data types differ",
						);
						validator.require(
							transfer.route.len() <= 1,
							ValidationCode::InvalidRoute,
							format!("{path}.route"),
							"executor-visible internal transfer must contain at most one directed link",
						);
						if let Err(errors) = topology.validate_route(source, destination, &transfer.route) {
							validator.append(&format!("{path}.route"), errors);
						}
						validate_submission(
							validator,
							&path,
							source,
							transfer.submission,
							&draft.resources,
						);
					}
					(TransferEndpoint::External, TransferEndpoint::External) => {
						validator.error(
							ValidationCode::InvalidExternalTransfer,
							format!("{path}.source"),
							"external-to-external transfer is not a run task",
						)
					}
				}
				validate_transfer_lane_claims(validator, &path, transfer, topology, discovery);
			}
			TaskKind::Metric(metric) => {
				validator.require(
					task.phase == RunPhase::Loop,
					ValidationCode::InvalidPhase,
					format!("{path}.phase"),
					"metric emission belongs to the loop",
				);
				validator.require(
					indexes.values.contains_key(&metric.value),
					ValidationCode::UnknownReference,
					format!("{path}.value"),
					format!("unknown value {}", metric.value),
				);
				if let Some(value) = indexes.values.get(&metric.value) {
					validate_submission(
						validator,
						&path,
						value.device,
						metric.submission,
						&draft.resources,
					);
				}
				validator.require(
					draft.resources
						.metric(metric.slot)
						.is_some_and(|slot| slot.metric == metric.metric),
					ValidationCode::UnknownReference,
					format!("{path}.slot"),
					"metric slot is absent or assigned to another metric",
				);
				if metric.purpose == MetricPurpose::FaultReadback {
					validator.require(
						indexes.values.get(&metric.value).is_some_and(|value| {
							value.dtype == DType::I32 && value.bytes == ByteCount::new(4)
						}),
						ValidationCode::InvalidFaultReadback,
						format!("{path}.value"),
						"fault readback value must be exactly one int32",
					);
				}
			}
		}
	}

	validate_fault_readbacks(validator, draft, &tasks);

	for (index, device) in topology.devices.iter().enumerate() {
		match upload_counts.get(&device.id).copied().unwrap_or(0) {
			0 => {
				validator.error(
					ValidationCode::MissingUpload,
					format!("topology.devices[{index}]"),
					format!("device {} has no init data-image admission", device.id),
				)
			}
			1 => {}
			count => {
				validator.error(
					ValidationCode::DuplicateUpload,
					format!("topology.devices[{index}]"),
					format!(
						"device {} has {count} init data-image admissions",
						device.id
					),
				)
			}
		}
	}

	let mut indegree = tasks
		.keys()
		.map(|id| (*id, 0usize))
		.collect::<BTreeMap<_, _>>();
	let mut edges = BTreeMap::<TaskId, Vec<TaskId>>::new();
	for task in tasks.values() {
		for dependency in &task.dependencies {
			if indegree.contains_key(dependency) {
				*indegree.get_mut(&task.id).expect("task key exists") += 1;
				edges.entry(*dependency).or_default().push(task.id);
			}
		}
	}
	let mut ready = indegree
		.iter()
		.filter_map(|(id, degree)| (*degree == 0).then_some(*id))
		.collect::<VecDeque<_>>();
	let mut visited = 0usize;
	while let Some(id) = ready.pop_front() {
		visited += 1;
		for successor in edges.get(&id).into_iter().flatten() {
			let degree = indegree.get_mut(successor).expect("successor key exists");
			*degree -= 1;
			if *degree == 0 {
				ready.push_back(*successor);
			}
		}
	}
	validator.require(
		visited == tasks.len(),
		ValidationCode::DependencyCycle,
		"tasks",
		"task dependency graph contains a cycle",
	);
	validate_resource_contention(validator, draft, topology, discovery);
}

fn validate_fault_readbacks(validator: &mut Validator, draft: &DraftPlan, tasks: &BTreeMap<TaskId, &Task>) {
	let mut checked_by_flag = BTreeMap::<ValueId, Vec<&Task>>::new();
	let mut readbacks_by_flag = BTreeMap::<ValueId, Vec<&Task>>::new();
	let mut metric_slot_uses = BTreeMap::new();
	for task in &draft.tasks {
		if let TaskKind::Calculation(calculation) = &task.kind
			&& let Some(fault_flag) = calculation.fault_flag
		{
			checked_by_flag.entry(fault_flag).or_default().push(task);
		}
		if let TaskKind::Metric(metric) = &task.kind {
			*metric_slot_uses.entry(metric.slot).or_insert(0usize) += 1;
			if metric.purpose == MetricPurpose::FaultReadback {
				readbacks_by_flag
					.entry(metric.value)
					.or_default()
					.push(task);
			}
		}
	}
	for (fault_flag, calculations) in &checked_by_flag {
		let readbacks = readbacks_by_flag.get(fault_flag);
		let readback_count = readbacks.map_or(0, Vec::len);
		validator.require(
			readback_count == 1,
			ValidationCode::InvalidFaultReadback,
			format!("values[{fault_flag}].fault_readback"),
			format!(
				"fault cohort with {} checked calculations requires exactly one readback, found {readback_count}",
				calculations.len()
			),
		);
		let Some(readback) = readbacks
			.and_then(|readbacks| readbacks.first().copied())
			.filter(|_| readback_count == 1)
		else {
			continue;
		};
		let TaskKind::Metric(metric) = &readback.kind else {
			continue;
		};
		let slot_uses = metric_slot_uses.get(&metric.slot).copied().unwrap_or(0);
		validator.require(
			slot_uses == 1,
			ValidationCode::InvalidFaultReadback,
			format!("tasks[{}].slot", readback.id),
			format!(
				"fault readback {} requires an exclusive metric slot, but slot {} has {slot_uses} users",
				readback.id, metric.slot
			),
		);

		for calculation in calculations {
			validator.require(
				readback.dependencies.contains(&calculation.id),
				ValidationCode::InvalidFaultReadback,
				format!("tasks[{}].dependencies", readback.id),
				format!(
					"fault readback {} must depend directly on checked calculation {}",
					readback.id, calculation.id
				),
			);
		}
		for published in draft.tasks.iter().filter(|task| {
			task.phase == RunPhase::Exit
				|| matches!(&task.kind, TaskKind::Metric(metric) if metric.purpose == MetricPurpose::User)
		}) {
			validator.require(
				task_depends_on(tasks, published.id, readback.id),
				ValidationCode::InvalidFaultReadback,
				format!("tasks[{}].dependencies", published.id),
				format!(
					"publishing task {} can complete before fault readback {}",
					published.id, readback.id
				),
			);
		}
	}
	for (fault_flag, readbacks) in &readbacks_by_flag {
		validator.require(
			checked_by_flag.contains_key(fault_flag),
			ValidationCode::InvalidFaultReadback,
			format!("tasks[{}].value", readbacks[0].id),
			format!("fault readback names unused flag {fault_flag}"),
		);
	}
}

fn task_depends_on(tasks: &BTreeMap<TaskId, &Task>, task: TaskId, ancestor: TaskId) -> bool {
	let Some(task) = tasks.get(&task).copied() else {
		return false;
	};
	let mut pending = task.dependencies.clone();
	let mut visited = BTreeSet::new();
	while let Some(dependency) = pending.pop() {
		if dependency == ancestor {
			return true;
		}
		if !visited.insert(dependency) {
			continue;
		}
		if let Some(predecessor) = tasks.get(&dependency) {
			pending.extend_from_slice(&predecessor.dependencies);
		}
	}
	false
}

fn validate_transfer_endpoint<'a>(
	validator: &mut Validator,
	task_path: &str,
	field: &str,
	endpoint: TransferEndpoint,
	bytes: ByteCount,
	values: &BTreeMap<ValueId, &'a ValueSpec>,
) -> Option<&'a ValueSpec> {
	let TransferEndpoint::Device { device, value } = endpoint else {
		return None;
	};
	let spec = values.get(&value).copied();
	validator.require(
		spec.is_some(),
		ValidationCode::UnknownReference,
		format!("{task_path}.{field}.value"),
		format!("unknown value {value}"),
	);
	match spec {
		Some(spec) => {
			validator.require(
				spec.device == device,
				ValidationCode::ResourceMismatch,
				format!("{task_path}.{field}.device"),
				format!(
					"value {} belongs to device {} rather than endpoint device {device}",
					spec.id, spec.device
				),
			);
			validator.require(
				bytes == spec.bytes,
				ValidationCode::ResourceMismatch,
				format!("{task_path}.bytes"),
				format!("transfer byte count differs from {field} value {value}"),
			);
			Some(spec)
		}
		None => None,
	}
}

fn validate_transfer_lane_claims(
	validator: &mut Validator,
	task_path: &str,
	transfer: &crate::schedule::TransferTask,
	topology: &Topology,
	discovery: &DiscoveryProfile,
) {
	validator.require(
		transfer
			.lane_claims
			.windows(2)
			.all(|claims| claims[0] < claims[1]),
		ValidationCode::InvalidLaneClaim,
		format!("{task_path}.lane_claims"),
		"transfer lane claims must be strictly sorted and unique",
	);
	let mut exact_claims = BTreeSet::new();
	let mut claimed_links = BTreeSet::new();
	let mut external_claims = Vec::new();
	for (index, claim) in transfer.lane_claims.iter().copied().enumerate() {
		let path = format!("{task_path}.lane_claims[{index}]");
		validator.require(
			exact_claims.insert(claim),
			ValidationCode::InvalidLaneClaim,
			&path,
			"transfer lane claim appears more than once",
		);
		match claim {
			TransferLaneClaim::Link { link, lane } => {
				validator.require(
					claimed_links.insert(link),
					ValidationCode::InvalidLaneClaim,
					&path,
					format!("directed link {link} has more than one lane claim"),
				);
				let topology_link = topology.link(link);
				validator.require(
					topology_link.is_some(),
					ValidationCode::InvalidLaneClaim,
					&path,
					format!("lane claim references unknown directed link {link}"),
				);
				if let Some(topology_link) = topology_link {
					validator.require(
						lane < topology_link.maximum_inflight_transfers.value.get(),
						ValidationCode::InvalidLaneClaim,
						&path,
						format!(
							"lane {lane} exceeds directed link {link} lane count {}",
							topology_link.maximum_inflight_transfers.value.get()
						),
					);
				}
			}
			TransferLaneClaim::External { device, lane } => {
				external_claims.push((device, lane));
				let discovered = discovery.device(device);
				validator.require(
					discovered.is_some(),
					ValidationCode::InvalidLaneClaim,
					&path,
					format!("external lane claim references unknown device {device}"),
				);
				if let Some(discovered) = discovered {
					validator.require(
						lane < discovered.transfer.maximum_inflight_transfers.value.get(),
						ValidationCode::InvalidLaneClaim,
						&path,
						format!(
							"lane {lane} exceeds device {device} external-transfer lane count {}",
							discovered.transfer.maximum_inflight_transfers.value.get()
						),
					);
				}
			}
		}
	}

	match (transfer.source, transfer.destination) {
		(TransferEndpoint::Device { .. }, TransferEndpoint::Device { .. }) => {
			let routed_links = transfer.route.iter().copied().collect::<BTreeSet<_>>();
			validator.require(
				external_claims.is_empty(),
				ValidationCode::InvalidLaneClaim,
				format!("{task_path}.lane_claims"),
				"internal transfer cannot claim an external device lane",
			);
			validator.require(
				claimed_links == routed_links,
				ValidationCode::InvalidLaneClaim,
				format!("{task_path}.lane_claims"),
				"internal transfer must claim exactly one lane for every unique route link",
			);
		}
		(TransferEndpoint::External, TransferEndpoint::Device { device, .. })
		| (TransferEndpoint::Device { device, .. }, TransferEndpoint::External) => {
			validator.require(
				claimed_links.is_empty(),
				ValidationCode::InvalidLaneClaim,
				format!("{task_path}.lane_claims"),
				"external transfer cannot claim an internal link lane",
			);
			validator.require(
				external_claims.len() == 1 && external_claims[0].0 == device,
				ValidationCode::InvalidLaneClaim,
				format!("{task_path}.lane_claims"),
				format!("external transfer must claim exactly one lane on endpoint device {device}"),
			);
		}
		(TransferEndpoint::External, TransferEndpoint::External) => {
			validator.require(
				transfer.lane_claims.is_empty(),
				ValidationCode::InvalidLaneClaim,
				format!("{task_path}.lane_claims"),
				"external-to-external transfer cannot claim a lane",
			);
		}
	}
}

fn validate_resource_contention(
	validator: &mut Validator,
	draft: &DraftPlan,
	topology: &Topology,
	discovery: &DiscoveryProfile,
) {
	for left_index in 0..draft.tasks.len() {
		for right_index in (left_index + 1)..draft.tasks.len() {
			let left = &draft.tasks[left_index];
			let right = &draft.tasks[right_index];
			if !left.window.overlaps(right.window) {
				continue;
			}
			if let (Some(left_slots), Some(right_slots)) =
				(submission_slots(&left.kind), submission_slots(&right.kind))
			{
				validator.require(
					left_slots.queue != right_slots.queue,
					ValidationCode::ResourceContention,
					format!("tasks[{right_index}].submission.queue"),
					format!(
						"overlapping tasks {} and {} share queue slot {}",
						left.id, right.id, right_slots.queue
					),
				);
				validator.require(
					left_slots.completion != right_slots.completion,
					ValidationCode::ResourceContention,
					format!("tasks[{right_index}].submission.completion"),
					format!(
						"overlapping tasks {} and {} share completion slot {}",
						left.id, right.id, right_slots.completion
					),
				);
			}

			if let (TaskKind::Transfer(left_transfer), TaskKind::Transfer(right_transfer)) =
				(&left.kind, &right.kind)
			{
				for claim in &left_transfer.lane_claims {
					if right_transfer.lane_claims.contains(claim) {
						validator.error(
							ValidationCode::ResourceContention,
							format!("tasks[{right_index}].lane_claims"),
							format!(
								"overlapping transfers {} and {} claim the same {claim:?}",
								left.id, right.id
							),
						);
					}
				}
				let left_links = transfer_links(left_transfer);
				let right_links = transfer_links(right_transfer);
				for first in &left_links {
					let Some(first_link) = topology.link(*first) else {
						continue;
					};
					if first_link.duplex != crate::topology::DuplexMode::Half {
						continue;
					}
					for second in &right_links {
						let Some(second_link) = topology.link(*second) else {
							continue;
						};
						if first_link.transport == second_link.transport && first_link.id != second_link.id
						{
							validator.error(
								ValidationCode::ResourceContention,
								format!("tasks[{right_index}].route"),
								format!(
									"opposing transfers {} and {} overlap on half-duplex transport {}",
									left.id, right.id, first_link.transport
								),
							);
						}
					}
				}
			}

			validate_compute_transfer_overlap(
				validator,
				left_index,
				left,
				right_index,
				right,
				topology,
				discovery,
			);
			validate_compute_transfer_overlap(
				validator,
				right_index,
				right,
				left_index,
				left,
				topology,
				discovery,
			);
		}
	}

	for link in &topology.links {
		let events = draft
			.tasks
			.iter()
			.filter_map(|task| {
				match &task.kind {
					TaskKind::Transfer(transfer) if transfer.route.contains(&link.id) => {
						Some([(task.window.start, 1i8), (task.window.end, -1i8)])
					}
					TaskKind::Calculation(_) | TaskKind::Transfer(_) | TaskKind::Metric(_) => None,
				}
			})
			.flatten()
			.collect::<Vec<_>>();
		validate_concurrency_events(
			validator,
			events,
			link.maximum_inflight_transfers.value.get(),
			format!("link {} directional transfer", link.id),
		);
	}

	for discovered in &discovery.devices {
		let events = draft
			.tasks
			.iter()
			.filter_map(|task| {
				match &task.kind {
					TaskKind::Transfer(transfer)
						if transfer.route.is_empty()
							&& matches!(
								(transfer.source, transfer.destination),
								(TransferEndpoint::External, TransferEndpoint::Device { device, .. })
									| (TransferEndpoint::Device { device, .. }, TransferEndpoint::External)
									if device == discovered.device
							) =>
					{
						Some([(task.window.start, 1i8), (task.window.end, -1i8)])
					}
					TaskKind::Calculation(_) | TaskKind::Transfer(_) | TaskKind::Metric(_) => None,
				}
			})
			.flatten()
			.collect::<Vec<_>>();
		validate_concurrency_events(
			validator,
			events,
			discovered.transfer.maximum_inflight_transfers.value.get(),
			format!("device {} external transfer", discovered.device),
		);
	}

	for discovered in &discovery.devices {
		let Some(calculation) = &discovered.calculation else {
			continue;
		};
		let mut events = draft
			.tasks
			.iter()
			.filter_map(|task| {
				match &task.kind {
					TaskKind::Calculation(candidate) if candidate.device == discovered.device => {
						Some([(task.window.start, 1i8), (task.window.end, -1i8)])
					}
					_ => None,
				}
			})
			.flatten()
			.collect::<Vec<_>>();
		events.sort_by_key(|(time, delta)| (*time, *delta));
		let mut active = 0u32;
		for (_, delta) in events {
			if delta < 0 {
				active = active.saturating_sub(1);
			} else {
				active = active.saturating_add(1);
				validator.require(
					active <= calculation.maximum_concurrent_tasks,
					ValidationCode::CapabilityConcurrencyExceeded,
					"tasks",
					format!(
						"device {} has {active} overlapping calculations but supports at most {}",
						discovered.device, calculation.maximum_concurrent_tasks
					),
				);
			}
		}
	}
}

fn validate_concurrency_events(
	validator: &mut Validator,
	mut events: Vec<(Nanoseconds, i8)>,
	maximum: u32,
	name: String,
) {
	events.sort_by_key(|(time, delta)| (*time, *delta));
	let mut active = 0u32;
	for (_, delta) in events {
		if delta < 0 {
			active = active.saturating_sub(1);
		} else {
			active = active.saturating_add(1);
			validator.require(
				active <= maximum,
				ValidationCode::CapabilityConcurrencyExceeded,
				"tasks",
				format!("{name} concurrency is {active}, but measured capacity is {maximum}"),
			);
		}
	}
}

fn submission_slots(kind: &TaskKind) -> Option<crate::schedule::SubmissionSlots> {
	match kind {
		TaskKind::Calculation(task) => Some(task.submission),
		TaskKind::Transfer(task) => Some(task.submission),
		TaskKind::Metric(task) => Some(task.submission),
	}
}

fn transfer_links(transfer: &crate::schedule::TransferTask) -> BTreeSet<crate::ids::LinkId> {
	transfer.route.iter().copied().collect()
}

fn validate_compute_transfer_overlap(
	validator: &mut Validator,
	calculation_index: usize,
	calculation_task: &Task,
	transfer_index: usize,
	transfer_task: &Task,
	topology: &Topology,
	discovery: &DiscoveryProfile,
) {
	let TaskKind::Calculation(calculation) = &calculation_task.kind else {
		return;
	};
	let TaskKind::Transfer(transfer) = &transfer_task.kind else {
		return;
	};
	let touches_device = match (transfer.source, transfer.destination) {
		(
			TransferEndpoint::Device { device: source, .. },
			TransferEndpoint::Device {
				device: destination,
				..
			},
		) => {
			source == calculation.device
				|| destination == calculation.device
				|| transfer.route.iter().any(|link| {
					topology.link(*link).is_some_and(|link| {
						link.from == calculation.device || link.to == calculation.device
					})
				})
		}
		(TransferEndpoint::External, TransferEndpoint::Device { device, .. })
		| (TransferEndpoint::Device { device, .. }, TransferEndpoint::External) => device == calculation.device,
		(TransferEndpoint::External, TransferEndpoint::External) => false,
	};
	if !touches_device {
		return;
	}
	let overlap_supported = discovery
		.device(calculation.device)
		.is_some_and(|device| device.transfer.overlaps_calculation);
	validator.require(
		overlap_supported,
		ValidationCode::ResourceContention,
		format!("tasks[{transfer_index}].window"),
		format!(
			"transfer task {} overlaps calculation task {} on device {} without overlap capability (calculation at tasks[{calculation_index}])",
			transfer_task.id, calculation_task.id, calculation.device
		),
	);
}

fn validate_submission(
	validator: &mut Validator,
	path: &str,
	device: DeviceId,
	submission: crate::schedule::SubmissionSlots,
	resources: &ResourceManifest,
) {
	validator.require(
		resources
			.queue(submission.queue)
			.is_some_and(|slot| slot.device == device),
		ValidationCode::ResourceMismatch,
		format!("{path}.submission.queue"),
		"queue slot is absent or belongs to another device",
	);
	validator.require(
		resources
			.completion(submission.completion)
			.is_some_and(|slot| slot.device == device),
		ValidationCode::ResourceMismatch,
		format!("{path}.submission.completion"),
		"completion slot is absent or belongs to another device",
	);
}

fn validate_arena_objects(validator: &mut Validator, draft: &DraftPlan, topology: &Topology) {
	let mut ids = BTreeSet::new();
	for (index, object) in draft.arena_objects.iter().enumerate() {
		validator.require(
			ids.insert(object.id),
			ValidationCode::DuplicateId,
			format!("arena_objects[{index}].id"),
			format!("arena object {} appears more than once", object.id),
		);
		validator.require(
			topology.device(object.device).is_some(),
			ValidationCode::UnknownReference,
			format!("arena_objects[{index}].device"),
			format!("unknown device {}", object.device),
		);
		validator.require(
			object.alignment.get() != 0 && object.alignment.get().is_power_of_two(),
			ValidationCode::AllocationMisaligned,
			format!("arena_objects[{index}].alignment"),
			"arena alignment must be a nonzero power of two",
		);
		validator.require(
			object.lifetime.is_valid(),
			ValidationCode::InvalidLifetime,
			format!("arena_objects[{index}].lifetime"),
			"arena object lifetime must be nonempty",
		);
	}
}

fn validate_value_bindings(
	validator: &mut Validator,
	draft: &DraftPlan,
	values: &BTreeMap<ValueId, &ValueSpec>,
	kernels: &BTreeMap<crate::ids::KernelTemplateId, &KernelTemplate>,
) {
	let objects = draft
		.arena_objects
		.iter()
		.map(|object| (object.id, object))
		.collect::<BTreeMap<_, _>>();
	let tasks = draft
		.tasks
		.iter()
		.map(|task| (task.id, task))
		.collect::<BTreeMap<_, _>>();
	let mut bindings = BTreeMap::<ValueId, &ValueBinding>::new();

	for (index, binding) in draft.value_bindings.iter().enumerate() {
		let path = format!("value_bindings[{index}]");
		validator.require(
			bindings.insert(binding.value, binding).is_none(),
			ValidationCode::DuplicateValueBinding,
			format!("{path}.value"),
			format!("value {} has more than one arena binding", binding.value),
		);
		let Some(value) = values.get(&binding.value).copied() else {
			validator.error(
				ValidationCode::UnknownReference,
				format!("{path}.value"),
				format!("unknown value {}", binding.value),
			);
			continue;
		};
		let Some(object) = objects.get(&binding.object).copied() else {
			validator.error(
				ValidationCode::UnknownReference,
				format!("{path}.object"),
				format!("unknown arena object {}", binding.object),
			);
			continue;
		};

		validator.require(
			value.device == object.device,
			ValidationCode::ResourceMismatch,
			format!("{path}.object"),
			format!(
				"value {} belongs to device {} but arena object {} belongs to {}",
				value.id, value.device, object.id, object.device
			),
		);
		let value_alignment = u64::from(value.dtype.byte_width());
		validator.require(
			object.alignment.get() >= value_alignment
				&& object.alignment.get() % value_alignment == 0
				&& binding.object_offset.get() % value_alignment == 0,
			ValidationCode::ValueBindingMisaligned,
			format!("{path}.object_offset"),
			format!(
				"value {} requires {value_alignment}-byte alignment",
				value.id
			),
		);
		validator.require(
			binding
				.object_offset
				.checked_end(value.bytes)
				.is_some_and(|end| end <= object.bytes),
			ValidationCode::ValueBindingOutOfBounds,
			format!("{path}.object_offset"),
			format!(
				"value {} does not fit within arena object {}",
				value.id, object.id
			),
		);

		if let Some(producer) = value.producer {
			let producer_task = tasks.get(&producer).copied();
			validator.require(
				producer_task.is_some(),
				ValidationCode::UnknownReference,
				format!("values[{}].producer", value.id),
				format!("unknown producer task {producer}"),
			);
			if let Some(producer_task) = producer_task {
				validator.require(
					window_contains(object.lifetime, producer_task.window),
					ValidationCode::ValueLifetimeMismatch,
					format!("{path}.object"),
					format!(
						"arena object {} lifetime does not contain producer task {}",
						object.id, producer
					),
				);
			}
		}

		for task in draft
			.tasks
			.iter()
			.filter(|task| task_references_value(task, value.id))
		{
			validator.require(
				window_contains(object.lifetime, task.window),
				ValidationCode::ValueLifetimeMismatch,
				format!("{path}.object"),
				format!(
					"arena object {} lifetime does not contain task {} using value {}",
					object.id, task.id, value.id
				),
			);
		}
	}

	for (index, value) in draft.values.iter().enumerate() {
		validator.require(
			bindings.contains_key(&value.id),
			ValidationCode::MissingValueBinding,
			format!("values[{index}]"),
			format!("value {} has no arena binding", value.id),
		);
	}

	validate_alias_bindings(validator, draft, values, kernels, &bindings);
}

fn validate_init_images(
	validator: &mut Validator,
	draft: &DraftPlan,
	topology: &Topology,
	values: &BTreeMap<ValueId, &ValueSpec>,
) {
	let bindings = draft
		.value_bindings
		.iter()
		.map(|binding| (binding.value, binding))
		.collect::<BTreeMap<_, _>>();
	let mut uploads = BTreeMap::new();
	for task in &draft.tasks {
		let TaskKind::Transfer(transfer) = &task.kind else {
			continue;
		};
		let (
			TransferEndpoint::External,
			TransferEndpoint::Device {
				device,
				value: image,
			},
		) = (transfer.source, transfer.destination)
		else {
			continue;
		};
		uploads
			.entry(device)
			.or_insert((task.id, image, transfer.bytes));
	}

	let mut image_devices = BTreeSet::new();
	let mut physical_members = BTreeSet::new();
	for (image_index, image) in draft.init_images.iter().enumerate() {
		let path = format!("init_images[{image_index}]");
		validator.require(
			topology.device(image.device).is_some(),
			ValidationCode::UnknownReference,
			format!("{path}.device"),
			format!("unknown device {}", image.device),
		);
		validator.require(
			image_devices.insert(image.device),
			ValidationCode::DuplicateId,
			format!("{path}.device"),
			format!("device {} has more than one init image", image.device),
		);
		validator.require(
			image.bytes != ByteCount::ZERO,
			ValidationCode::InvalidDataImage,
			format!("{path}.bytes"),
			"init image must contain at least one byte",
		);

		let image_spec = values.get(&image.image).copied();
		validator.require(
			image_spec.is_some(),
			ValidationCode::UnknownReference,
			format!("{path}.image"),
			format!("unknown image value {}", image.image),
		);
		if let Some(image_spec) = image_spec {
			validator.require(
				image_spec.device == image.device && image_spec.bytes == image.bytes,
				ValidationCode::ResourceMismatch,
				format!("{path}.image"),
				"image value does not match the manifest device and exact byte size",
			);
		}

		let upload = uploads.get(&image.device).copied();
		validator.require(
			upload.is_some(),
			ValidationCode::MissingUpload,
			format!("{path}.device"),
			format!("device {} has no init admission task", image.device),
		);
		if let Some((upload_task, upload_value, upload_bytes)) = upload {
			validator.require(
				upload_value == image.image && upload_bytes == image.bytes,
				ValidationCode::InvalidDataImage,
				path.clone(),
				"init image does not exactly match the device's sole external admission",
			);
			if let Some(image_spec) = image_spec {
				validator.require(
					image_spec.producer == Some(upload_task),
					ValidationCode::InvalidDataImage,
					format!("{path}.image"),
					"image value is not produced by its external admission task",
				);
			}
		}

		let image_binding = bindings.get(&image.image).copied();
		validator.require(
			image_binding.is_some(),
			ValidationCode::MissingValueBinding,
			format!("{path}.image"),
			"init image has no arena binding",
		);
		let mut logical_members = BTreeSet::new();
		let mut occupied = Vec::new();
		for (member_index, member) in image.members.iter().enumerate() {
			let member_path = format!("{path}.members[{member_index}]");
			validator.require(
				member.logical.get() != 0,
				ValidationCode::InvalidDataImage,
				format!("{member_path}.logical"),
				"logical input value identity must not be zero",
			);
			validator.require(
				logical_members.insert(member.logical),
				ValidationCode::DuplicateId,
				format!("{member_path}.logical"),
				format!(
					"logical input {} appears more than once in device {} init image",
					member.logical, image.device
				),
			);
			validator.require(
				physical_members.insert(member.physical),
				ValidationCode::DuplicateId,
				format!("{member_path}.physical"),
				format!(
					"physical input {} appears in more than one init-image member",
					member.physical
				),
			);
			validator.require(
				member.image_offset.get() % u64::from(member.dtype.byte_width()) == 0,
				ValidationCode::ValueBindingMisaligned,
				format!("{member_path}.image_offset"),
				"init-image member offset does not satisfy its scalar alignment",
			);
			let member_end = member.image_offset.checked_end(member.bytes);
			validator.require(
				member_end.is_some_and(|end| end <= image.bytes),
				ValidationCode::ValueBindingOutOfBounds,
				format!("{member_path}.image_offset"),
				"init-image member exceeds the packed image",
			);
			if let Some(end) = member_end {
				occupied.push((member.image_offset.get(), end.get(), member_index));
			}

			let physical_spec = values.get(&member.physical).copied();
			validator.require(
				physical_spec.is_some(),
				ValidationCode::UnknownReference,
				format!("{member_path}.physical"),
				format!("unknown physical value {}", member.physical),
			);
			if let Some(physical_spec) = physical_spec {
				validator.require(
					physical_spec.device == image.device
						&& physical_spec.dtype == member.dtype
						&& physical_spec.bytes == member.bytes,
					ValidationCode::ResourceMismatch,
					member_path.clone(),
					"init-image member does not match its physical value contract",
				);
				if let Some((upload_task, _, _)) = upload {
					validator.require(
						physical_spec.producer == Some(upload_task),
						ValidationCode::InvalidDataImage,
						format!("{member_path}.physical"),
						"physical member is not produced by this image's admission task",
					);
				}
			}

			let physical_binding = bindings.get(&member.physical).copied();
			validator.require(
				physical_binding.is_some(),
				ValidationCode::MissingValueBinding,
				format!("{member_path}.physical"),
				"physical member has no arena binding",
			);
			if let (Some(image_binding), Some(physical_binding)) = (image_binding, physical_binding) {
				validator.require(
					image_binding.object == physical_binding.object,
					ValidationCode::ResourceMismatch,
					format!("{member_path}.physical"),
					"physical member is not stored in the init image's arena object",
				);
				let expected_offset = image_binding
					.object_offset
					.get()
					.checked_add(member.image_offset.get());
				validator.require(
					expected_offset == Some(physical_binding.object_offset.get()),
					ValidationCode::InvalidDataImage,
					format!("{member_path}.image_offset"),
					"member image offset does not resolve to its physical arena binding",
				);
			}
		}

		occupied.sort_unstable();
		for pair in occupied.windows(2) {
			let [left, right] = pair else {
				continue;
			};
			validator.require(
				left.1 <= right.0,
				ValidationCode::LiveAllocationOverlap,
				format!("{path}.members[{}]", right.2),
				format!(
					"init-image member overlaps members[{}] in the packed image",
					left.2
				),
			);
		}
	}

	for (device_index, device) in topology.devices.iter().enumerate() {
		validator.require(
			image_devices.contains(&device.id),
			ValidationCode::MissingRequiredObject,
			format!("topology.devices[{device_index}]"),
			format!("required device {} has no init-image manifest", device.id),
		);
	}
}

fn validate_alias_bindings(
	validator: &mut Validator,
	draft: &DraftPlan,
	values: &BTreeMap<ValueId, &ValueSpec>,
	kernels: &BTreeMap<crate::ids::KernelTemplateId, &KernelTemplate>,
	bindings: &BTreeMap<ValueId, &ValueBinding>,
) {
	let mut source_pairs = BTreeSet::new();
	for (alias_index, alias) in draft.value_aliases.iter().enumerate() {
		let path = format!("value_aliases[{alias_index}]");
		validator.require(
			source_pairs.insert((alias.input, alias.output)),
			ValidationCode::DuplicateAliasRule,
			&path,
			format!(
				"value alias pair {} -> {} appears more than once",
				alias.input, alias.output
			),
		);
		let input_binding = bindings.get(&alias.input).copied();
		let output_binding = bindings.get(&alias.output).copied();
		let input_spec = values.get(&alias.input).copied();
		let output_spec = values.get(&alias.output).copied();
		validator.require(
			input_binding.is_some() && input_spec.is_some(),
			ValidationCode::UnknownReference,
			format!("{path}.input"),
			format!("unknown input value {}", alias.input),
		);
		validator.require(
			output_binding.is_some() && output_spec.is_some(),
			ValidationCode::UnknownReference,
			format!("{path}.output"),
			format!("unknown output value {}", alias.output),
		);
		if let (Some(input_binding), Some(output_binding), Some(input_spec), Some(output_spec)) =
			(input_binding, output_binding, input_spec, output_spec)
		{
			validate_alias_pair(
				validator,
				&path,
				alias.permission,
				AliasValue {
					id: alias.input,
					binding: input_binding,
					spec: input_spec,
				},
				AliasValue {
					id: alias.output,
					binding: output_binding,
					spec: output_spec,
				},
			);
		}
	}
	for (task_index, task) in draft.tasks.iter().enumerate() {
		let TaskKind::Calculation(calculation) = &task.kind else {
			continue;
		};
		let Some(kernel) = kernels.get(&calculation.kernel_template).copied() else {
			continue;
		};
		validator.require(
			calculation.inputs.len() == kernel.inputs.len(),
			ValidationCode::ScalarArity,
			format!("tasks[{task_index}].inputs"),
			"calculation input count differs from its kernel template",
		);
		validator.require(
			calculation.outputs.len() == kernel.outputs.len(),
			ValidationCode::ScalarArity,
			format!("tasks[{task_index}].outputs"),
			"calculation output count differs from its kernel template",
		);

		for rule in &kernel.alias_rules {
			let Some(input_index) = kernel
				.inputs
				.iter()
				.position(|input| input.id == rule.input)
			else {
				continue;
			};
			let Some(output_index) = kernel
				.outputs
				.iter()
				.position(|output| output.id == rule.output)
			else {
				continue;
			};
			let (Some(input_value), Some(output_value)) = (
				calculation.inputs.get(input_index).copied(),
				calculation.outputs.get(output_index).copied(),
			) else {
				continue;
			};
			let (Some(input_binding), Some(output_binding), Some(input_spec), Some(output_spec)) = (
				bindings.get(&input_value).copied(),
				bindings.get(&output_value).copied(),
				values.get(&input_value).copied(),
				values.get(&output_value).copied(),
			) else {
				continue;
			};
			validate_alias_pair(
				validator,
				&format!("tasks[{task_index}].alias[{}->{}]", rule.input, rule.output),
				rule.permission,
				AliasValue {
					id: input_value,
					binding: input_binding,
					spec: input_spec,
				},
				AliasValue {
					id: output_value,
					binding: output_binding,
					spec: output_spec,
				},
			);
		}
	}
}

#[derive(Clone, Copy, Debug)]
struct AliasValue<'a> {
	id: ValueId,
	binding: &'a ValueBinding,
	spec: &'a ValueSpec,
}

fn validate_alias_pair(
	validator: &mut Validator,
	path: &str,
	permission: AliasPermission,
	input: AliasValue<'_>,
	output: AliasValue<'_>,
) {
	let exact = input.binding.object == output.binding.object
		&& input.binding.object_offset == output.binding.object_offset
		&& input.spec.bytes == output.spec.bytes;
	let overlaps = binding_ranges_overlap(
		input.binding,
		input.spec.bytes,
		output.binding,
		output.spec.bytes,
	);
	let valid = match permission {
		AliasPermission::Forbidden => !overlaps,
		AliasPermission::MayAliasExact => !overlaps || exact,
		AliasPermission::MustAliasExact => exact,
	};
	validator.require(
		valid,
		ValidationCode::AliasViolation,
		path,
		format!(
			"values {} and {} violate {permission:?}",
			input.id, output.id
		),
	);
}

fn binding_ranges_overlap(
	left: &ValueBinding,
	left_bytes: ByteCount,
	right: &ValueBinding,
	right_bytes: ByteCount,
) -> bool {
	if left.object != right.object {
		return false;
	}
	let (Some(left_end), Some(right_end)) = (
		left.object_offset.checked_end(left_bytes),
		right.object_offset.checked_end(right_bytes),
	) else {
		return true;
	};
	ByteCount::new(left.object_offset.get()) < right_end && ByteCount::new(right.object_offset.get()) < left_end
}

fn task_references_value(task: &Task, value: ValueId) -> bool {
	match &task.kind {
		TaskKind::Calculation(calculation) => {
			calculation.inputs.contains(&value)
				|| calculation.outputs.contains(&value)
				|| calculation.fault_flag == Some(value)
		}
		TaskKind::Transfer(transfer) => {
			transfer.source.value() == Some(value) || transfer.destination.value() == Some(value)
		}
		TaskKind::Metric(metric) => metric.value == value,
	}
}

fn window_contains(container: crate::schedule::ScheduleWindow, contained: crate::schedule::ScheduleWindow) -> bool {
	container.start <= contained.start && contained.end <= container.end
}

fn validate_releases(validator: &mut Validator, draft: &DraftPlan, topology: &Topology) {
	let mut counts = BTreeMap::<DeviceId, usize>::new();
	for (index, release) in draft.releases.iter().enumerate() {
		validator.require(
			topology.device(release.device).is_some(),
			ValidationCode::UnknownReference,
			format!("releases[{index}].device"),
			format!("unknown device {}", release.device),
		);
		*counts.entry(release.device).or_default() += 1;
	}
	for (index, device) in topology.devices.iter().enumerate() {
		match counts.get(&device.id).copied().unwrap_or(0) {
			0 => {
				validator.error(
					ValidationCode::MissingRelease,
					format!("topology.devices[{index}]"),
					format!("device {} has no exit arena release", device.id),
				)
			}
			1 => {}
			count => {
				validator.error(
					ValidationCode::DuplicateRelease,
					format!("topology.devices[{index}]"),
					format!("device {} has {count} exit arena releases", device.id),
				)
			}
		}
	}
}

/// Stable output of Realize, keyed to one unchanged Draft.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RealizationProfile {
	pub identity: RealizationIdentity,
	pub draft: DraftIdentity,
	pub candidate: CandidateIdentity,
	pub discovery: DiscoveryIdentity,
	pub topology: TopologyIdentity,
	pub artifacts: Vec<ArtifactIdentity>,
	pub resources: ResourceManifest,
	pub reservations: ReservationLedger,
	pub capacity: CapacityLedger,
}

impl RealizationProfile {
	pub fn validate(&self, topology: &Topology, discovery: &DiscoveryProfile, draft: &DraftPlan) -> ValidationResult {
		let mut validator = Validator::default();
		validator.require(
			!self.identity.is_zero(),
			ValidationCode::InvalidIdentity,
			"identity",
			"realization identity must not be zero",
		);
		for (path, matches, message) in [
			(
				"draft",
				self.draft == draft.identity,
				"draft identity changed",
			),
			(
				"candidate",
				self.candidate == draft.candidate,
				"candidate identity changed",
			),
			(
				"discovery",
				self.discovery == discovery.identity,
				"discovery identity changed",
			),
			(
				"topology",
				self.topology == topology.identity,
				"topology identity changed",
			),
		] {
			validator.require(matches, ValidationCode::IdentityMismatch, path, message);
		}
		validate_realized_artifacts(&mut validator, self, draft, discovery);
		validator.require(
			self.resources == draft.resources,
			ValidationCode::ResourceMismatch,
			"resources",
			"realization resources differ from the unchanged draft",
		);
		if let Err(errors) = self.reservations.validate(topology) {
			validator.append("reservations", errors);
		}
		if let Err(errors) = self.capacity.validate(topology, &self.reservations) {
			validator.append("capacity", errors);
		}
		validator.finish()
	}
}

fn validate_realized_artifacts(
	validator: &mut Validator,
	realization: &RealizationProfile,
	draft: &DraftPlan,
	discovery: &DiscoveryProfile,
) {
	let realized = realization
		.artifacts
		.iter()
		.map(|artifact| (artifact.id, artifact))
		.collect::<BTreeMap<_, _>>();
	validator.require(
		realized.len() == realization.artifacts.len(),
		ValidationCode::DuplicateId,
		"artifacts",
		"realization artifact IDs must be unique",
	);
	let drafted = draft
		.artifacts
		.iter()
		.map(|artifact| (artifact.id, artifact))
		.collect::<BTreeMap<_, _>>();
	let builds = draft
		.artifact_builds
		.iter()
		.map(|build| (build.artifact, build))
		.collect::<BTreeMap<_, _>>();
	validator.require(
		realized.len() == drafted.len().saturating_add(builds.len()),
		ValidationCode::ArtifactMismatch,
		"artifacts",
		"realization must contain exactly every prebuilt and deferred artifact",
	);
	for (id, expected) in drafted {
		validator.require(
			realized.get(&id).copied() == Some(expected),
			ValidationCode::ArtifactMismatch,
			format!("artifacts[{id}]"),
			"prebuilt Draft artifact changed during realization",
		);
	}
	for (id, build) in builds {
		let actual = realized.get(&id).copied();
		validator.require(
			actual.is_some(),
			ValidationCode::ArtifactMismatch,
			format!("artifacts[{id}]"),
			"deferred Draft artifact was not realized",
		);
		if let Some(actual) = actual {
			if let Err(errors) = actual.validate() {
				validator.append(&format!("artifacts[{id}]"), errors);
			}
			validator.require(
				actual.kernel_template == build.kernel_template,
				ValidationCode::ArtifactMismatch,
				format!("artifacts[{id}].kernel_template"),
				"realized artifact names a different stage",
			);
			validator.require(
				actual.build == Some(build.provenance),
				ValidationCode::ArtifactMismatch,
				format!("artifacts[{id}].build"),
				"realized artifact does not prove the exact deferred contract",
			);
			validator.require(
				actual.resources == build.resources,
				ValidationCode::ResourceMismatch,
				format!("artifacts[{id}].resources"),
				"realized resource envelope differs from Draft",
			);
			for task in &draft.tasks {
				let TaskKind::Calculation(calculation) = &task.kind else {
					continue;
				};
				if calculation.artifact != id {
					continue;
				}
				validator.require(
					discovery
						.device(calculation.device)
						.and_then(|device| device.calculation.as_ref())
						.is_some_and(|capability| capability.target == actual.target),
					ValidationCode::ArtifactMismatch,
					format!("artifacts[{id}].target"),
					format!(
						"realized target does not match task {} placement on device {}",
						task.id, calculation.device
					),
				);
			}
		}
	}
	for artifact in &realization.artifacts {
		validator.require(
			draft.artifacts
				.iter()
				.any(|expected| expected.id == artifact.id)
				|| draft
					.artifact_builds
					.iter()
					.any(|build| build.artifact == artifact.id),
			ValidationCode::ArtifactMismatch,
			format!("artifacts[{}]", artifact.id),
			"realization contains an artifact absent from Draft",
		);
	}
}

fn validate_loop_domains(
	validator: &mut Validator,
	tasks: &[Task],
	loop_iterations: LoopIterations,
	loop_domains: &[LoopTaskDomain],
) {
	let tasks_by_id = tasks
		.iter()
		.map(|task| (task.id, task))
		.collect::<BTreeMap<_, _>>();
	let mut domains = BTreeMap::new();
	for (index, assignment) in loop_domains.iter().enumerate() {
		let path = format!("loop_domains[{index}]");
		validator.require(
			domains.insert(assignment.task, assignment.domain).is_none(),
			ValidationCode::DuplicateId,
			format!("{path}.task"),
			format!(
				"task {} has more than one iteration domain",
				assignment.task
			),
		);
		let task = tasks_by_id.get(&assignment.task).copied();
		validator.require(
			task.is_some(),
			ValidationCode::UnknownReference,
			format!("{path}.task"),
			format!(
				"iteration domain references unknown task {}",
				assignment.task
			),
		);
		validator.require(
			task.is_some_and(|task| task.phase == RunPhase::Loop),
			ValidationCode::InvalidPhase,
			format!("{path}.task"),
			"only loop tasks may have an iteration domain",
		);
		validator.require(
			assignment.domain.is_within(loop_iterations),
			ValidationCode::InvalidIterationDomain,
			format!("{path}.domain"),
			format!(
				"iteration domain [{}, {}) stride {} exceeds loop [0, {})",
				assignment.domain.first_iteration(),
				assignment
					.domain
					.end_exclusive()
					.map_or_else(|| "unbounded".to_owned(), |end| end.to_string()),
				assignment.domain.stride(),
				loop_iterations
					.finite()
					.map_or_else(|| "unbounded".to_owned(), |end| end.get().to_string())
			),
		);
	}
	for (index, task) in tasks.iter().enumerate() {
		validator.require(
			(task.phase == RunPhase::Loop) == domains.contains_key(&task.id),
			ValidationCode::InvalidIterationDomain,
			format!("tasks[{index}].iteration_domain"),
			match task.phase {
				RunPhase::Loop => "loop task has no iteration domain",
				RunPhase::Init | RunPhase::Exit => "non-loop task has an iteration domain",
			},
		);
	}

	for (index, task) in tasks.iter().enumerate() {
		let Some(task_domain) = domains.get(&task.id).copied() else {
			continue;
		};
		if let TaskKind::Metric(metric) = &task.kind
			&& metric.purpose == MetricPurpose::FaultReadback
		{
			for checked in tasks.iter().filter(|candidate| {
				matches!(
					&candidate.kind,
					TaskKind::Calculation(calculation) if calculation.fault_flag == Some(metric.value)
				)
			}) {
				if let Some(calculation_domain) = domains.get(&checked.id).copied() {
					validator.require(
						task_domain == calculation_domain,
						ValidationCode::InvalidIterationDomain,
						format!("tasks[{index}].iteration_domain"),
						format!(
							"fault readback {} must share checked calculation {}'s iteration domain",
							task.id, checked.id
						),
					);
				}
			}
		}

		let TaskKind::Transfer(transfer) = &task.kind else {
			continue;
		};
		let (
			TransferEndpoint::Device {
				value: destination, ..
			},
			TransferEndpoint::Device { .. },
		) = (transfer.destination, transfer.source)
		else {
			continue;
		};
		for (consumer_index, consumer) in tasks.iter().enumerate() {
			if consumer.id == task.id
				|| consumer.phase != RunPhase::Loop
				|| !task_reads_value(consumer, destination)
			{
				continue;
			}
			if let Some(consumer_domain) = domains.get(&consumer.id).copied() {
				validator.require(
					task_domain == consumer_domain,
					ValidationCode::InvalidIterationDomain,
					format!("tasks[{consumer_index}].iteration_domain"),
					format!(
						"consumer {} must share internal transfer {}'s iteration domain for value {destination}",
						consumer.id, task.id
					),
				);
			}
		}
	}
}

fn task_reads_value(task: &Task, value: ValueId) -> bool {
	match &task.kind {
		TaskKind::Calculation(calculation) => calculation.inputs.contains(&value),
		TaskKind::Transfer(transfer) => {
			matches!(
				transfer.source,
				TransferEndpoint::Device {
					value: source,
					..
				} if source == value
			)
		}
		TaskKind::Metric(metric) => metric.value == value,
	}
}

/// Immutable product of Finalize. Fields can only be populated by validation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FinalizedBundle {
	identity: BundleIdentity,
	loop_iterations: LoopIterations,
	loop_domains: Vec<LoopTaskDomain>,
	topology: TopologyIdentity,
	discovery: DiscoveryIdentity,
	draft: DraftIdentity,
	realization: RealizationIdentity,
	candidate: CandidateIdentity,
	artifacts: Vec<ArtifactIdentity>,
	artifact_builds: Vec<ArtifactBuildRecipe>,
	kernels: Vec<KernelTemplate>,
	tasks: Vec<Task>,
	resources: ResourceManifest,
	reservations: ReservationLedger,
	arena_layouts: Vec<ArenaLayout>,
	value_locations: Vec<ResolvedValueLocation>,
	value_aliases: Vec<ValueAliasContract>,
	init_images: Vec<InitDataImage>,
}

impl FinalizedBundle {
	pub fn finalize(
		identity: BundleIdentity,
		topology: &Topology,
		discovery: &DiscoveryProfile,
		draft: DraftPlan,
		realization: RealizationProfile,
		arena_layouts: Vec<ArenaLayout>,
	) -> ValidationResult<Self> {
		Self::finalize_with_loop_iterations(
			identity,
			topology,
			discovery,
			draft,
			realization,
			arena_layouts,
			LoopIterations::ONE,
		)
	}

	/// Finalizes one immutable bundle with an explicit bounded loop count.
	///
	/// The loop graph, artifacts, arenas, and pending-token slots remain
	/// singular; only execution of the already-finalized loop is repeated.
	pub fn finalize_with_loop_iterations(
		identity: BundleIdentity,
		topology: &Topology,
		discovery: &DiscoveryProfile,
		draft: DraftPlan,
		realization: RealizationProfile,
		arena_layouts: Vec<ArenaLayout>,
		loop_iterations: LoopIterations,
	) -> ValidationResult<Self> {
		let loop_domains = draft
			.tasks
			.iter()
			.filter(|task| task.phase == RunPhase::Loop)
			.map(|task| {
				LoopTaskDomain {
					task: task.id,
					domain: IterationDomain::every(loop_iterations),
				}
			})
			.collect();
		Self::finalize_with_loop_schedule(
			identity,
			topology,
			discovery,
			draft,
			realization,
			arena_layouts,
			LoopSchedule::new(loop_iterations, loop_domains),
		)
	}

	/// Finalizes one immutable bundle with an exact activation domain for every
	/// loop task.
	pub fn finalize_with_loop_schedule(
		identity: BundleIdentity,
		topology: &Topology,
		discovery: &DiscoveryProfile,
		draft: DraftPlan,
		realization: RealizationProfile,
		arena_layouts: Vec<ArenaLayout>,
		loop_schedule: LoopSchedule,
	) -> ValidationResult<Self> {
		let (loop_iterations, mut loop_domains) = loop_schedule.into_parts();
		loop_domains.sort_by_key(|entry| entry.task);
		let mut validator = Validator::default();
		validator.require(
			!identity.is_zero(),
			ValidationCode::InvalidIdentity,
			"identity",
			"bundle identity must not be zero",
		);
		if let Err(errors) = draft.validate(topology, discovery) {
			validator.append("draft", errors);
		}
		if let Err(errors) = realization.validate(topology, discovery, &draft) {
			validator.append("realization", errors);
		}
		validate_loop_domains(&mut validator, &draft.tasks, loop_iterations, &loop_domains);
		validate_layouts(
			&mut validator,
			topology,
			&draft.arena_objects,
			&realization.capacity,
			&arena_layouts,
		);
		let value_locations = resolve_value_locations(
			&mut validator,
			&draft.values,
			&draft.value_bindings,
			&draft.arena_objects,
			&arena_layouts,
		);
		if let Some(errors) = validator.into_errors() {
			return Err(errors);
		}
		Ok(Self {
			identity,
			loop_iterations,
			loop_domains,
			topology: topology.identity,
			discovery: discovery.identity,
			draft: draft.identity,
			realization: realization.identity,
			candidate: draft.candidate,
			artifacts: realization.artifacts,
			artifact_builds: draft.artifact_builds,
			kernels: draft.kernels,
			tasks: draft.tasks,
			resources: draft.resources,
			reservations: realization.reservations,
			arena_layouts,
			value_locations,
			value_aliases: draft.value_aliases,
			init_images: draft.init_images,
		})
	}

	#[must_use]
	pub const fn identity(&self) -> BundleIdentity { self.identity }

	#[must_use]
	pub const fn loop_iterations(&self) -> LoopIterations { self.loop_iterations }

	#[must_use]
	pub fn loop_domains(&self) -> &[LoopTaskDomain] { &self.loop_domains }

	#[must_use]
	pub fn iteration_domain(&self, task: TaskId) -> Option<IterationDomain> {
		self.loop_domains
			.binary_search_by_key(&task, |entry| entry.task)
			.ok()
			.map(|index| self.loop_domains[index].domain)
	}

	#[must_use]
	pub const fn topology(&self) -> TopologyIdentity { self.topology }

	#[must_use]
	pub const fn discovery(&self) -> DiscoveryIdentity { self.discovery }

	#[must_use]
	pub const fn draft(&self) -> DraftIdentity { self.draft }

	#[must_use]
	pub const fn realization(&self) -> RealizationIdentity { self.realization }

	#[must_use]
	pub const fn candidate(&self) -> CandidateIdentity { self.candidate }

	#[must_use]
	pub fn artifacts(&self) -> &[ArtifactIdentity] { &self.artifacts }

	#[must_use]
	pub fn artifact_builds(&self) -> &[ArtifactBuildRecipe] { &self.artifact_builds }

	#[must_use]
	pub fn artifact_build(&self, artifact: ArtifactId) -> Option<&ArtifactBuildRecipe> {
		self.artifact_builds
			.iter()
			.find(|build| build.artifact == artifact)
	}

	#[must_use]
	pub fn kernels(&self) -> &[KernelTemplate] { &self.kernels }

	#[must_use]
	pub fn kernel(&self, kernel: crate::ids::KernelTemplateId) -> Option<&KernelTemplate> {
		self.kernels.iter().find(|candidate| candidate.id == kernel)
	}

	#[must_use]
	pub fn tasks(&self) -> &[Task] { &self.tasks }

	#[must_use]
	pub const fn resources(&self) -> &ResourceManifest { &self.resources }

	#[must_use]
	pub const fn reservations(&self) -> &ReservationLedger { &self.reservations }

	#[must_use]
	pub fn arena_layouts(&self) -> &[ArenaLayout] { &self.arena_layouts }

	#[must_use]
	pub fn value_locations(&self) -> &[ResolvedValueLocation] { &self.value_locations }

	#[must_use]
	pub fn value_aliases(&self) -> &[ValueAliasContract] { &self.value_aliases }

	#[must_use]
	pub fn init_images(&self) -> &[InitDataImage] { &self.init_images }

	#[must_use]
	pub fn init_image(&self, device: DeviceId) -> Option<&InitDataImage> {
		self.init_images.iter().find(|image| image.device == device)
	}

	#[must_use]
	pub fn value_location(&self, value: ValueId) -> Option<&ResolvedValueLocation> {
		self.value_locations
			.iter()
			.find(|location| location.value == value)
	}

	#[must_use]
	pub fn transfer_endpoints(&self, task: TaskId) -> Option<ResolvedTransferEndpoints> {
		let TaskKind::Transfer(transfer) = &self
			.tasks
			.iter()
			.find(|candidate| candidate.id == task)?
			.kind
		else {
			return None;
		};
		Some(ResolvedTransferEndpoints {
			source: self.resolve_transfer_endpoint(transfer.source)?,
			destination: self.resolve_transfer_endpoint(transfer.destination)?,
		})
	}

	fn resolve_transfer_endpoint(&self, endpoint: TransferEndpoint) -> Option<ResolvedTransferEndpoint> {
		match endpoint {
			TransferEndpoint::External => Some(ResolvedTransferEndpoint::External),
			TransferEndpoint::Device { device, value } => {
				let location = self.value_location(value).copied()?;
				(location.device == device).then_some(ResolvedTransferEndpoint::Device(location))
			}
		}
	}
}

fn validate_layouts(
	validator: &mut Validator,
	topology: &Topology,
	objects: &[ArenaObject],
	capacity: &CapacityLedger,
	layouts: &[ArenaLayout],
) {
	let object_map = objects
		.iter()
		.map(|object| (object.id, object))
		.collect::<BTreeMap<ArenaObjectId, _>>();
	let mut allocated = BTreeSet::new();
	let mut layout_devices = BTreeSet::new();

	for (layout_index, layout) in layouts.iter().enumerate() {
		let path = format!("arena_layouts[{layout_index}]");
		validator.require(
			topology.device(layout.device).is_some(),
			ValidationCode::UnknownReference,
			format!("{path}.device"),
			format!("unknown device {}", layout.device),
		);
		validator.require(
			layout_devices.insert(layout.device),
			ValidationCode::DuplicateId,
			format!("{path}.device"),
			format!("device {} has more than one arena layout", layout.device),
		);
		let mut maximum_end = ByteCount::ZERO;
		for (allocation_index, allocation) in layout.allocations.iter().enumerate() {
			let allocation_path = format!("{path}.allocations[{allocation_index}]");
			validator.require(
				allocated.insert(allocation.object),
				ValidationCode::DuplicateAllocation,
				format!("{allocation_path}.object"),
				format!(
					"arena object {} is allocated more than once",
					allocation.object
				),
			);
			let Some(object) = object_map.get(&allocation.object).copied() else {
				validator.error(
					ValidationCode::UnknownReference,
					format!("{allocation_path}.object"),
					format!("unknown arena object {}", allocation.object),
				);
				continue;
			};
			validator.require(
				object.device == layout.device,
				ValidationCode::ResourceMismatch,
				allocation_path.clone(),
				"arena object is allocated on the wrong device",
			);
			validator.require(
				allocation.offset.get() % object.alignment.get() == 0,
				ValidationCode::AllocationMisaligned,
				format!("{allocation_path}.offset"),
				"arena allocation does not satisfy its alignment",
			);
			match allocation.offset.checked_end(object.bytes) {
				Some(end) => {
					maximum_end = maximum_end.max(end);
					validator.require(
						end <= layout.size,
						ValidationCode::AllocationOutOfBounds,
						allocation_path,
						"arena allocation exceeds its layout size",
					);
				}
				None => {
					validator.error(
						ValidationCode::AllocationOutOfBounds,
						allocation_path,
						"arena allocation end overflowed",
					)
				}
			}
		}
		validator.require(
			layout.size == maximum_end,
			ValidationCode::ResourceMismatch,
			format!("{path}.size"),
			"arena size must equal the exact maximum allocation end",
		);
		if let Some(entry) = capacity.entry(layout.device) {
			validator.require(
				layout.size <= entry.recipe_usable.value,
				ValidationCode::InsufficientCapacity,
				format!("{path}.size"),
				"finalized arena exceeds Recipe-usable capacity",
			);
		}

		for left_index in 0..layout.allocations.len() {
			for right_index in (left_index + 1)..layout.allocations.len() {
				let left_allocation = layout.allocations[left_index];
				let right_allocation = layout.allocations[right_index];
				let (Some(left), Some(right)) = (
					object_map.get(&left_allocation.object).copied(),
					object_map.get(&right_allocation.object).copied(),
				) else {
					continue;
				};
				let Some(left_end) = left_allocation.offset.checked_end(left.bytes) else {
					continue;
				};
				let Some(right_end) = right_allocation.offset.checked_end(right.bytes) else {
					continue;
				};
				let memory_overlaps = ByteCount::new(left_allocation.offset.get()) < right_end
					&& ByteCount::new(right_allocation.offset.get()) < left_end;
				validator.require(
					!(memory_overlaps && left.lifetime.overlaps(right.lifetime)),
					ValidationCode::LiveAllocationOverlap,
					path.clone(),
					format!("live arena objects {} and {} overlap", left.id, right.id),
				);
			}
		}
	}

	for (index, device) in topology.devices.iter().enumerate() {
		validator.require(
			layout_devices.contains(&device.id),
			ValidationCode::MissingRequiredObject,
			format!("topology.devices[{index}]"),
			format!(
				"required device {} has no finalized arena layout",
				device.id
			),
		);
	}
	for (index, object) in objects.iter().enumerate() {
		validator.require(
			allocated.contains(&object.id),
			ValidationCode::MissingAllocation,
			format!("arena_objects[{index}]"),
			format!("arena object {} has no finalized allocation", object.id),
		);
	}
}

fn resolve_value_locations(
	validator: &mut Validator,
	values: &[ValueSpec],
	bindings: &[ValueBinding],
	objects: &[ArenaObject],
	layouts: &[ArenaLayout],
) -> Vec<ResolvedValueLocation> {
	let values_by_id = values
		.iter()
		.map(|value| (value.id, value))
		.collect::<BTreeMap<_, _>>();
	let objects_by_id = objects
		.iter()
		.map(|object| (object.id, object))
		.collect::<BTreeMap<_, _>>();
	let mut bindings_by_value = BTreeMap::new();
	for binding in bindings {
		bindings_by_value.entry(binding.value).or_insert(binding);
	}
	let mut allocations = BTreeMap::new();
	for layout in layouts {
		for allocation in &layout.allocations {
			allocations
				.entry(allocation.object)
				.or_insert((layout.device, layout.size, allocation.offset));
		}
	}

	let mut locations = Vec::with_capacity(values.len());
	for value in values_by_id.values().copied() {
		let Some(binding) = bindings_by_value.get(&value.id).copied() else {
			continue;
		};
		let Some(object) = objects_by_id.get(&binding.object).copied() else {
			continue;
		};
		let Some((layout_device, layout_size, object_offset)) = allocations.get(&binding.object).copied() else {
			continue;
		};
		let Some(arena_offset_raw) = object_offset.get().checked_add(binding.object_offset.get()) else {
			validator.error(
				ValidationCode::AddressOverflow,
				format!("value_bindings[{}]", value.id),
				format!(
					"final arena offset overflows for value {} in object {}",
					value.id, object.id
				),
			);
			continue;
		};
		let arena_offset = crate::units::ByteOffset::new(arena_offset_raw);
		let Some(arena_end) = arena_offset.checked_end(value.bytes) else {
			validator.error(
				ValidationCode::AddressOverflow,
				format!("value_bindings[{}]", value.id),
				format!("final arena end overflows for value {}", value.id),
			);
			continue;
		};
		validator.require(
			layout_device == value.device,
			ValidationCode::ResourceMismatch,
			format!("value_bindings[{}]", value.id),
			"resolved value location belongs to the wrong device arena",
		);
		validator.require(
			arena_end <= layout_size,
			ValidationCode::ValueBindingOutOfBounds,
			format!("value_bindings[{}]", value.id),
			"resolved value location exceeds the finalized arena",
		);
		validator.require(
			arena_offset
				.get()
				.is_multiple_of(u64::from(value.dtype.byte_width())),
			ValidationCode::ValueBindingMisaligned,
			format!("value_bindings[{}]", value.id),
			"resolved value location is not aligned for its payload type",
		);
		locations.push(ResolvedValueLocation {
			value: value.id,
			dtype: value.dtype,
			device: value.device,
			bytes: value.bytes,
			object: object.id,
			object_offset: binding.object_offset,
			arena_offset,
		});
	}
	locations.sort_by_key(|location| location.value);
	locations
}
