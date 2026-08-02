use std::{
	collections::{BTreeMap, BTreeSet},
	error::Error,
	fmt,
};

use recipe_core::{
	ArtifactId, BundleIdentity, ByteCount, DType, DeviceId, Digest, FinalizedBundle, KernelTemplateId, LinkId,
	LoopIteration, MachineId, MetricId, MetricPurpose, MetricSlotId, NodeId, NodeRole, ResolvedTransferEndpoint,
	ResolvedValueLocation, RunId, RunPhase, ScheduleWindow, SubmissionSlots, Task, TaskId, TaskKind, Topology,
	TopologyIdentity, TransferLaneClaim,
};
use sha2::{Digest as _, Sha256};

use crate::{
	backend::{
		ArenaSet, Backend, BackendPoll, BackendWork, CalculationWork, InitAdmissionWork, MetricWork,
		PendingRequest, PhysicalCallBatch, TransferWork, WorkClass,
	},
	executor::{JournalCapacity, LogicalEvent, RunJournal, Watchdog},
	metrics::MetricValue,
};

/// Exact machine/node ownership selected for one remote worker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkerAssignment {
	machine: MachineId,
	node: NodeId,
}

impl WorkerAssignment {
	#[must_use]
	pub const fn new(machine: MachineId, node: NodeId) -> Self { Self { machine, node } }

	#[must_use]
	pub const fn machine(self) -> MachineId { self.machine }

	#[must_use]
	pub const fn node(self) -> NodeId { self.node }
}

/// Closed ownership role of one task in a worker projection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkerTaskRole {
	InitAdmission,
	Local,
	ExternalIngress,
	ExternalEgress,
}

/// Direction of a cross-machine byte transfer relative to the worker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExternalTransferDirection {
	Ingress,
	Egress,
}

/// Exact preprovisioned cross-machine transfer handed to a worker backend.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkerExternalTransfer {
	task: TaskId,
	phase: RunPhase,
	direction: ExternalTransferDirection,
	local: ResolvedValueLocation,
	bytes: ByteCount,
	route: Box<[LinkId]>,
	lane_claims: Box<[TransferLaneClaim]>,
	submission: SubmissionSlots,
}

impl WorkerExternalTransfer {
	#[must_use]
	pub const fn task(&self) -> TaskId { self.task }

	#[must_use]
	pub const fn phase(&self) -> RunPhase { self.phase }

	#[must_use]
	pub const fn direction(&self) -> ExternalTransferDirection { self.direction }

	#[must_use]
	pub const fn local(&self) -> ResolvedValueLocation { self.local }

	#[must_use]
	pub const fn bytes(&self) -> ByteCount { self.bytes }

	#[must_use]
	pub fn route(&self) -> &[LinkId] { &self.route }

	#[must_use]
	pub fn lane_claims(&self) -> &[TransferLaneClaim] { &self.lane_claims }

	#[must_use]
	pub const fn submission(&self) -> SubmissionSlots { self.submission }
}

/// A projection cannot be derived without preserving these global contracts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum WorkerProjectionError {
	InvalidTopology,
	TopologyMismatch {
		bundle: TopologyIdentity,
		actual: TopologyIdentity,
	},
	UnknownMachine(MachineId),
	UnknownNode(NodeId),
	NodeIsNotWorker(NodeId),
	NodeMachineMismatch {
		node: NodeId,
		expected: MachineId,
		actual: MachineId,
	},
	EmptyWorker(NodeId),
	MissingArena(DeviceId),
	MissingReservation(DeviceId),
	MissingInitImage(DeviceId),
	MissingInitAdmission(DeviceId),
	DuplicateInitAdmission(DeviceId),
	InvalidTask {
		task: TaskId,
		detail: &'static str,
	},
	InvalidResource {
		task: TaskId,
		detail: &'static str,
	},
	CapacityOverflow,
}

impl fmt::Display for WorkerProjectionError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::InvalidTopology => formatter.write_str("worker projection requires a valid measured topology"),
			Self::TopologyMismatch { bundle, actual } => {
				write!(
					formatter,
					"worker topology {actual:?} differs from bundle topology {bundle:?}"
				)
			}
			Self::UnknownMachine(machine) => write!(formatter, "worker machine {machine} is unknown"),
			Self::UnknownNode(node) => write!(formatter, "worker node {node} is unknown"),
			Self::NodeIsNotWorker(node) => write!(formatter, "node {node} is not a worker"),
			Self::NodeMachineMismatch {
				node,
				expected,
				actual,
			} => {
				write!(
					formatter,
					"worker node {node} belongs to machine {actual}, expected {expected}"
				)
			}
			Self::EmptyWorker(node) => write!(formatter, "worker node {node} owns no devices"),
			Self::MissingArena(device) => write!(formatter, "worker device {device} has no finalized arena"),
			Self::MissingReservation(device) => {
				write!(
					formatter,
					"worker device {device} has no finalized user reservation"
				)
			}
			Self::MissingInitImage(device) => {
				write!(
					formatter,
					"worker device {device} has no finalized init image"
				)
			}
			Self::MissingInitAdmission(device) => {
				write!(
					formatter,
					"worker device {device} has no exact init admission"
				)
			}
			Self::DuplicateInitAdmission(device) => {
				write!(
					formatter,
					"worker device {device} has multiple init admissions"
				)
			}
			Self::InvalidTask { task, detail } => write!(formatter, "worker task {task} is invalid: {detail}"),
			Self::InvalidResource { task, detail } => {
				write!(
					formatter,
					"worker task {task} has an invalid resource: {detail}"
				)
			}
			Self::CapacityOverflow => formatter.write_str("worker projection capacity arithmetic overflowed"),
		}
	}
}

impl Error for WorkerProjectionError {}

#[derive(Clone, Debug)]
enum PreparedWorkerWork {
	InitAdmission {
		device: DeviceId,
		destination: ResolvedValueLocation,
		bytes: ByteCount,
		submission: SubmissionSlots,
	},
	Calculation {
		device: DeviceId,
		kernel_template: KernelTemplateId,
		artifact: ArtifactId,
		submission: SubmissionSlots,
		inputs: Box<[ResolvedValueLocation]>,
		outputs: Box<[ResolvedValueLocation]>,
		fault_flag: Option<ResolvedValueLocation>,
	},
	InternalTransfer {
		source: ResolvedTransferEndpoint,
		destination: ResolvedTransferEndpoint,
		bytes: ByteCount,
		route: Box<[LinkId]>,
		lane_claims: Box<[TransferLaneClaim]>,
		submission: SubmissionSlots,
		class: WorkClass,
	},
	Metric {
		purpose: MetricPurpose,
		metric: MetricId,
		slot: MetricSlotId,
		value: ResolvedValueLocation,
		submission: SubmissionSlots,
	},
	External(WorkerExternalTransfer),
}

impl PreparedWorkerWork {
	const fn role(&self) -> WorkerTaskRole {
		match self {
			Self::InitAdmission { .. } => WorkerTaskRole::InitAdmission,
			Self::Calculation { .. } | Self::InternalTransfer { .. } | Self::Metric { .. } => {
				WorkerTaskRole::Local
			}
			Self::External(transfer) => {
				match transfer.direction {
					ExternalTransferDirection::Ingress => WorkerTaskRole::ExternalIngress,
					ExternalTransferDirection::Egress => WorkerTaskRole::ExternalEgress,
				}
			}
		}
	}

	const fn class(&self) -> Option<WorkClass> {
		match self {
			Self::InitAdmission { .. } => Some(WorkClass::InitAdmission),
			Self::Calculation { .. } => Some(WorkClass::Calculation),
			Self::InternalTransfer { class, .. } => Some(*class),
			Self::Metric { .. } => Some(WorkClass::Metric),
			Self::External(_) => None,
		}
	}

	const fn submission(&self) -> Option<SubmissionSlots> {
		match self {
			Self::InitAdmission { submission, .. }
			| Self::Calculation { submission, .. }
			| Self::InternalTransfer { submission, .. }
			| Self::Metric { submission, .. } => Some(*submission),
			Self::External(_) => None,
		}
	}

	fn backend_work<'a>(
		&'a self,
		task: TaskId,
		run: RunId,
		iteration: Option<LoopIteration>,
		image: Option<&'a [u8]>,
	) -> Option<BackendWork<'a>> {
		match self {
			Self::InitAdmission {
				destination,
				bytes,
				submission,
				..
			} => {
				image.map(|image| {
					BackendWork::InitAdmission(InitAdmissionWork {
						task,
						destination: *destination,
						bytes: *bytes,
						submission: *submission,
						image,
					})
				})
			}
			Self::Calculation {
				device,
				kernel_template,
				artifact,
				submission,
				inputs,
				outputs,
				fault_flag,
			} => {
				Some(BackendWork::Calculation(CalculationWork {
					task,
					run,
					iteration: iteration?,
					device: *device,
					kernel_template: *kernel_template,
					artifact: *artifact,
					submission: *submission,
					inputs,
					outputs,
					fault_flag: *fault_flag,
				}))
			}
			Self::InternalTransfer {
				source,
				destination,
				bytes,
				route,
				lane_claims,
				submission,
				class,
			} => {
				let work = TransferWork {
					task,
					source: *source,
					destination: *destination,
					bytes: *bytes,
					route,
					lane_claims,
					submission: *submission,
				};
				match class {
					WorkClass::InternalTransfer => Some(BackendWork::InternalTransfer(work)),
					WorkClass::ExitTransfer => Some(BackendWork::ExitTransfer(work)),
					WorkClass::InitAdmission | WorkClass::Calculation | WorkClass::Metric => None,
				}
			}
			Self::Metric {
				purpose,
				metric,
				slot,
				value,
				submission,
			} => {
				Some(BackendWork::Metric(MetricWork {
					task,
					iteration: iteration?,
					purpose: *purpose,
					metric: *metric,
					slot: *slot,
					value: *value,
					submission: *submission,
				}))
			}
			Self::External(_) => None,
		}
	}
}

#[derive(Clone, Debug)]
struct ProjectedTask {
	id: TaskId,
	phase: RunPhase,
	window: ScheduleWindow,
	dependencies: Box<[TaskId]>,
	projected_dependencies: Box<[TaskId]>,
	work: PreparedWorkerWork,
}

impl ProjectedTask {
	const fn role(&self) -> WorkerTaskRole { self.work.role() }
}

/// Immutable, validated task/device/resource projection for one worker node.
#[derive(Clone, Debug)]
pub struct WorkerProjection {
	identity: Digest,
	bundle: BundleIdentity,
	topology: TopologyIdentity,
	assignment: WorkerAssignment,
	devices: Box<[DeviceId]>,
	layouts: Box<[recipe_core::ArenaLayout]>,
	tasks: Box<[ProjectedTask]>,
	artifacts: Box<[ArtifactId]>,
}

impl WorkerProjection {
	pub fn derive(
		bundle: &FinalizedBundle,
		topology: &Topology,
		assignment: WorkerAssignment,
	) -> Result<Self, WorkerProjectionError> {
		topology
			.validate()
			.and_then(|()| topology.validate_scheduling_properties())
			.map_err(|errors| {
				drop(errors);
				WorkerProjectionError::InvalidTopology
			})?;
		if bundle.topology() != topology.identity {
			return Err(WorkerProjectionError::TopologyMismatch {
				bundle: bundle.topology(),
				actual: topology.identity,
			});
		}
		if topology
			.machines
			.iter()
			.all(|machine| machine.id != assignment.machine)
		{
			return Err(WorkerProjectionError::UnknownMachine(assignment.machine));
		}
		let node = topology
			.nodes
			.iter()
			.find(|node| node.id == assignment.node)
			.ok_or(WorkerProjectionError::UnknownNode(assignment.node))?;
		if node.role != NodeRole::Worker {
			return Err(WorkerProjectionError::NodeIsNotWorker(node.id));
		}
		if node.machine != assignment.machine {
			return Err(WorkerProjectionError::NodeMachineMismatch {
				node: node.id,
				expected: assignment.machine,
				actual: node.machine,
			});
		}
		if node.devices.is_empty() {
			return Err(WorkerProjectionError::EmptyWorker(node.id));
		}
		let mut devices = node.devices.clone();
		devices.sort_unstable();
		let local = devices.iter().copied().collect::<BTreeSet<_>>();
		let mut layouts = Vec::with_capacity(devices.len());
		for device in &devices {
			let layout = bundle
				.arena_layouts()
				.iter()
				.find(|layout| layout.device == *device)
				.ok_or(WorkerProjectionError::MissingArena(*device))?;
			if bundle.reservations().entry(*device).is_none() {
				return Err(WorkerProjectionError::MissingReservation(*device));
			}
			if bundle.init_image(*device).is_none() {
				return Err(WorkerProjectionError::MissingInitImage(*device));
			}
			layouts.push(layout.clone());
		}

		let mut classified = BTreeMap::<TaskId, Option<PreparedWorkerWork>>::new();
		for task in bundle.tasks() {
			let work = classify_task(bundle, topology, task, &local)?;
			if classified.insert(task.id, work).is_some() {
				return Err(WorkerProjectionError::InvalidTask {
					task: task.id,
					detail: "task identity appears more than once",
				});
			}
		}

		let projected_ids = classified
			.iter()
			.filter_map(|(task, work)| work.as_ref().map(|_| *task))
			.collect::<BTreeSet<_>>();
		let mut tasks = Vec::with_capacity(projected_ids.len());
		for task in bundle.tasks() {
			let Some(work) = classified.get_mut(&task.id).and_then(Option::take) else {
				continue;
			};
			let projected_dependencies = task
				.dependencies
				.iter()
				.copied()
				.filter(|dependency| projected_ids.contains(dependency))
				.collect::<Vec<_>>()
				.into_boxed_slice();
			validate_task_resources(bundle, task, &work, &local)?;
			tasks.push(ProjectedTask {
				id: task.id,
				phase: task.phase,
				window: task.window,
				dependencies: task.dependencies.clone().into_boxed_slice(),
				projected_dependencies,
				work,
			});
		}
		tasks.sort_by_key(|task| task.id);

		for device in &devices {
			let mut admissions = tasks.iter().filter(|task| {
				matches!(
				task.work,
				PreparedWorkerWork::InitAdmission {
					device: candidate,
					..
				} if candidate == *device
				)
			});
			let admission = match (admissions.next(), admissions.next()) {
				(None, _) => return Err(WorkerProjectionError::MissingInitAdmission(*device)),
				(Some(_), Some(_)) => return Err(WorkerProjectionError::DuplicateInitAdmission(*device)),
				(Some(admission), None) => admission,
			};
			let image = bundle
				.init_image(*device)
				.ok_or(WorkerProjectionError::MissingInitImage(*device))?;
			let PreparedWorkerWork::InitAdmission {
				destination, bytes, ..
			} = admission.work
			else {
				return Err(WorkerProjectionError::MissingInitAdmission(*device));
			};
			if destination.value != image.image || bytes != image.bytes {
				return Err(WorkerProjectionError::InvalidTask {
					task: admission.id,
					detail: "worker admission differs from the finalized init-image packing",
				});
			}
		}
		if let Some(task) = tasks.iter().find(|task| {
			task.phase == RunPhase::Init && !matches!(task.work, PreparedWorkerWork::InitAdmission { .. })
		}) {
			return Err(WorkerProjectionError::InvalidTask {
				task: task.id,
				detail: "remote worker init may contain only its exact device admissions",
			});
		}

		let artifacts = tasks
			.iter()
			.filter_map(|task| {
				match task.work {
					PreparedWorkerWork::Calculation { artifact, .. } => Some(artifact),
					_ => None,
				}
			})
			.collect::<BTreeSet<_>>()
			.into_iter()
			.collect::<Vec<_>>()
			.into_boxed_slice();
		let identity = projection_digest(bundle, topology, assignment, &devices, &tasks);
		Ok(Self {
			identity,
			bundle: bundle.identity(),
			topology: topology.identity,
			assignment,
			devices: devices.into_boxed_slice(),
			layouts: layouts.into_boxed_slice(),
			tasks: tasks.into_boxed_slice(),
			artifacts,
		})
	}

	#[must_use]
	pub const fn identity(&self) -> Digest { self.identity }

	#[must_use]
	pub const fn bundle(&self) -> BundleIdentity { self.bundle }

	#[must_use]
	pub const fn topology(&self) -> TopologyIdentity { self.topology }

	#[must_use]
	pub const fn assignment(&self) -> WorkerAssignment { self.assignment }

	#[must_use]
	pub fn devices(&self) -> &[DeviceId] { &self.devices }

	#[must_use]
	pub fn layouts(&self) -> &[recipe_core::ArenaLayout] { &self.layouts }

	pub fn tasks(&self) -> impl ExactSizeIterator<Item = (TaskId, RunPhase, WorkerTaskRole)> + '_ {
		self.tasks
			.iter()
			.map(|task| (task.id, task.phase, task.role()))
	}

	#[must_use]
	pub fn artifacts(&self) -> &[ArtifactId] { &self.artifacts }

	#[must_use]
	pub fn init_image_bytes(&self, device: DeviceId) -> Option<ByteCount> {
		self.tasks.iter().find_map(|task| {
			match task.work {
				PreparedWorkerWork::InitAdmission {
					device: candidate,
					bytes,
					..
				} if candidate == device => Some(bytes),
				_ => None,
			}
		})
	}

	pub fn external_transfers(&self) -> impl Iterator<Item = &WorkerExternalTransfer> {
		self.tasks.iter().filter_map(|task| {
			match &task.work {
				PreparedWorkerWork::External(transfer) => Some(transfer),
				_ => None,
			}
		})
	}

	#[must_use]
	pub fn task_role(&self, id: TaskId) -> Option<WorkerTaskRole> {
		self.tasks
			.binary_search_by_key(&id, |task| task.id)
			.ok()
			.map(|index| self.tasks[index].role())
	}

	#[must_use]
	pub fn task_phase(&self, id: TaskId) -> Option<RunPhase> {
		self.tasks
			.binary_search_by_key(&id, |task| task.id)
			.ok()
			.map(|index| self.tasks[index].phase)
	}
}

fn classify_task(
	bundle: &FinalizedBundle,
	topology: &Topology,
	task: &Task,
	local: &BTreeSet<DeviceId>,
) -> Result<Option<PreparedWorkerWork>, WorkerProjectionError> {
	match &task.kind {
		TaskKind::Calculation(calculation) => {
			if !local.contains(&calculation.device) {
				return Ok(None);
			}
			let inputs = resolve_local_values(bundle, task.id, &calculation.inputs, local)?;
			let outputs = resolve_local_values(bundle, task.id, &calculation.outputs, local)?;
			let fault_flag = calculation
				.fault_flag
				.map(|value| resolve_local_value(bundle, task.id, value, local))
				.transpose()?;
			Ok(Some(PreparedWorkerWork::Calculation {
				device: calculation.device,
				kernel_template: calculation.kernel_template,
				artifact: calculation.artifact,
				submission: calculation.submission,
				inputs,
				outputs,
				fault_flag,
			}))
		}
		TaskKind::Metric(metric) => {
			let location =
				bundle.value_location(metric.value)
					.copied()
					.ok_or(WorkerProjectionError::InvalidTask {
						task: task.id,
						detail: "metric value has no finalized location",
					})?;
			match local.contains(&location.device) {
				true => {
					Ok(Some(PreparedWorkerWork::Metric {
						purpose: metric.purpose,
						metric: metric.metric,
						slot: metric.slot,
						value: location,
						submission: metric.submission,
					}))
				}
				false => Ok(None),
			}
		}
		TaskKind::Transfer(transfer) => {
			let endpoints = bundle
				.transfer_endpoints(task.id)
				.ok_or(WorkerProjectionError::InvalidTask {
					task: task.id,
					detail: "transfer endpoints are not finalized",
				})?;
			let source_local = endpoint_is_local(endpoints.source, local);
			let destination_local = endpoint_is_local(endpoints.destination, local);
			match (
				endpoints.source,
				endpoints.destination,
				source_local,
				destination_local,
			) {
				(
					ResolvedTransferEndpoint::External,
					ResolvedTransferEndpoint::Device(destination),
					false,
					true,
				) => {
					if task.phase != RunPhase::Init {
						return Err(WorkerProjectionError::InvalidTask {
							task: task.id,
							detail: "external worker admission is legal only during init",
						});
					}
					Ok(Some(PreparedWorkerWork::InitAdmission {
						device: destination.device,
						destination,
						bytes: transfer.bytes,
						submission: transfer.submission,
					}))
				}
				(ResolvedTransferEndpoint::Device(source), ResolvedTransferEndpoint::External, true, false) => {
					if task.phase != RunPhase::Exit {
						return Err(WorkerProjectionError::InvalidTask {
							task: task.id,
							detail: "external worker egress is legal only during exit",
						});
					}
					Ok(Some(PreparedWorkerWork::External(external_transfer(
						topology,
						task,
						transfer,
						ExternalTransferDirection::Egress,
						source,
					)?)))
				}
				(ResolvedTransferEndpoint::Device(_), ResolvedTransferEndpoint::External, false, false)
				| (ResolvedTransferEndpoint::External, ResolvedTransferEndpoint::Device(_), false, false) => Ok(None),
				(
					ResolvedTransferEndpoint::Device(source),
					ResolvedTransferEndpoint::Device(destination),
					true,
					true,
				) => {
					let class = match task.phase {
						RunPhase::Init | RunPhase::Loop => WorkClass::InternalTransfer,
						RunPhase::Exit => WorkClass::ExitTransfer,
					};
					Ok(Some(PreparedWorkerWork::InternalTransfer {
						source: ResolvedTransferEndpoint::Device(source),
						destination: ResolvedTransferEndpoint::Device(destination),
						bytes: transfer.bytes,
						route: transfer.route.clone().into_boxed_slice(),
						lane_claims: transfer.lane_claims.clone().into_boxed_slice(),
						submission: transfer.submission,
						class,
					}))
				}
				(
					ResolvedTransferEndpoint::Device(source),
					ResolvedTransferEndpoint::Device(_),
					true,
					false,
				) => {
					Ok(Some(PreparedWorkerWork::External(external_transfer(
						topology,
						task,
						transfer,
						ExternalTransferDirection::Egress,
						source,
					)?)))
				}
				(
					ResolvedTransferEndpoint::Device(_),
					ResolvedTransferEndpoint::Device(destination),
					false,
					true,
				) => {
					Ok(Some(PreparedWorkerWork::External(external_transfer(
						topology,
						task,
						transfer,
						ExternalTransferDirection::Ingress,
						destination,
					)?)))
				}
				(ResolvedTransferEndpoint::Device(_), ResolvedTransferEndpoint::Device(_), false, false) => {
					Ok(None)
				}
				_ => {
					Err(WorkerProjectionError::InvalidTask {
						task: task.id,
						detail: "transfer endpoint ownership is not representable",
					})
				}
			}
		}
	}
}

fn external_transfer(
	topology: &Topology,
	task: &Task,
	transfer: &recipe_core::TransferTask,
	direction: ExternalTransferDirection,
	local: ResolvedValueLocation,
) -> Result<WorkerExternalTransfer, WorkerProjectionError> {
	if task.phase == RunPhase::Init || transfer.route.len() != 1 {
		return Err(WorkerProjectionError::InvalidTask {
			task: task.id,
			detail: "cross-machine transfers must be planner-expanded one-hop loop or exit tasks",
		});
	}
	let link = topology
		.link(transfer.route[0])
		.ok_or(WorkerProjectionError::InvalidTask {
			task: task.id,
			detail: "cross-machine route names an unknown measured link",
		})?;
	let local_matches = match direction {
		ExternalTransferDirection::Ingress => link.to == local.device,
		ExternalTransferDirection::Egress => link.from == local.device,
	};
	if !local_matches {
		return Err(WorkerProjectionError::InvalidTask {
			task: task.id,
			detail: "cross-machine route direction differs from the worker endpoint",
		});
	}
	let mut link_claims = transfer.lane_claims.iter().filter_map(|claim| {
		match claim {
			TransferLaneClaim::Link { link, lane } => Some((*link, *lane)),
			TransferLaneClaim::External { .. } => None,
		}
	});
	let Some((claimed_link, lane)) = link_claims.next() else {
		return Err(WorkerProjectionError::InvalidResource {
			task: task.id,
			detail: "cross-machine transfer has no link lane claim",
		});
	};
	if claimed_link != link.id
		|| link_claims.next().is_some()
		|| transfer
			.lane_claims
			.iter()
			.any(|claim| matches!(claim, TransferLaneClaim::External { .. }))
		|| lane >= link.maximum_inflight_transfers.value.get()
	{
		return Err(WorkerProjectionError::InvalidResource {
			task: task.id,
			detail: "cross-machine transfer lane claims differ from its measured route",
		});
	}
	Ok(WorkerExternalTransfer {
		task: task.id,
		phase: task.phase,
		direction,
		local,
		bytes: transfer.bytes,
		route: transfer.route.clone().into_boxed_slice(),
		lane_claims: transfer.lane_claims.clone().into_boxed_slice(),
		submission: transfer.submission,
	})
}

fn endpoint_is_local(endpoint: ResolvedTransferEndpoint, local: &BTreeSet<DeviceId>) -> bool {
	match endpoint {
		ResolvedTransferEndpoint::External => false,
		ResolvedTransferEndpoint::Device(location) => local.contains(&location.device),
	}
}

fn resolve_local_values(
	bundle: &FinalizedBundle,
	task: TaskId,
	values: &[recipe_core::ValueId],
	local: &BTreeSet<DeviceId>,
) -> Result<Box<[ResolvedValueLocation]>, WorkerProjectionError> {
	values.iter()
		.map(|value| resolve_local_value(bundle, task, *value, local))
		.collect::<Result<Vec<_>, _>>()
		.map(Vec::into_boxed_slice)
}

fn resolve_local_value(
	bundle: &FinalizedBundle,
	task: TaskId,
	value: recipe_core::ValueId,
	local: &BTreeSet<DeviceId>,
) -> Result<ResolvedValueLocation, WorkerProjectionError> {
	let location = bundle
		.value_location(value)
		.copied()
		.ok_or(WorkerProjectionError::InvalidTask {
			task,
			detail: "task value has no finalized location",
		})?;
	if !local.contains(&location.device) {
		return Err(WorkerProjectionError::InvalidTask {
			task,
			detail: "local worker task references a foreign value",
		});
	}
	Ok(location)
}

fn validate_task_resources(
	bundle: &FinalizedBundle,
	task: &Task,
	work: &PreparedWorkerWork,
	local: &BTreeSet<DeviceId>,
) -> Result<(), WorkerProjectionError> {
	let submission = match work {
		PreparedWorkerWork::External(transfer) => Some(transfer.submission),
		_ => work.submission(),
	};
	if let Some(submission) = submission {
		let queue = bundle
			.resources()
			.queue(submission.queue)
			.ok_or(WorkerProjectionError::InvalidResource {
				task: task.id,
				detail: "submission queue is absent",
			})?;
		let completion = bundle.resources().completion(submission.completion).ok_or(
			WorkerProjectionError::InvalidResource {
				task: task.id,
				detail: "completion slot is absent",
			},
		)?;
		if queue.device != completion.device {
			return Err(WorkerProjectionError::InvalidResource {
				task: task.id,
				detail: "worker submission queue and completion belong to different devices",
			});
		}
		let owner_is_valid = match work {
			PreparedWorkerWork::External(transfer) => {
				let TaskKind::Transfer(task_transfer) = &task.kind else {
					return Err(WorkerProjectionError::InvalidResource {
						task: task.id,
						detail: "external worker work is not a transfer task",
					});
				};
				let expected = match transfer.direction {
					ExternalTransferDirection::Ingress => {
						match task_transfer.source {
							recipe_core::TransferEndpoint::Device { device, .. } => device,
							recipe_core::TransferEndpoint::External => {
								return Err(WorkerProjectionError::InvalidResource {
									task: task.id,
									detail: "cross-machine ingress has no source submission device",
								});
							}
						}
					}
					ExternalTransferDirection::Egress => transfer.local.device,
				};
				queue.device == expected
			}
			_ => local.contains(&queue.device),
		};
		if !owner_is_valid {
			return Err(WorkerProjectionError::InvalidResource {
				task: task.id,
				detail: "worker submission resource belongs to the wrong endpoint device",
			});
		}
	}
	if let PreparedWorkerWork::Metric { metric, slot, .. } = work
		&& !bundle
			.resources()
			.metric(*slot)
			.is_some_and(|resource| resource.metric == *metric)
	{
		return Err(WorkerProjectionError::InvalidResource {
			task: task.id,
			detail: "metric slot is absent",
		});
	}
	Ok(())
}

fn projection_digest(
	bundle: &FinalizedBundle,
	topology: &Topology,
	assignment: WorkerAssignment,
	devices: &[DeviceId],
	tasks: &[ProjectedTask],
) -> Digest {
	let mut hash = Sha256::new();
	hash.update(b"recipe-worker-projection-v1");
	hash.update(bundle.identity().digest().bytes());
	hash.update(topology.identity.digest().bytes());
	hash.update(assignment.machine.get().to_le_bytes());
	hash.update(assignment.node.get().to_le_bytes());
	hash.update((devices.len() as u64).to_le_bytes());
	for device in devices {
		hash.update(device.get().to_le_bytes());
	}
	hash.update((tasks.len() as u64).to_le_bytes());
	for task in tasks {
		hash.update(task.id.get().to_le_bytes());
		hash.update([phase_code(task.phase), role_code(task.role())]);
		hash.update((task.dependencies.len() as u64).to_le_bytes());
		for dependency in &task.dependencies {
			hash.update(dependency.get().to_le_bytes());
		}
	}
	Digest::new(hash.finalize().into())
}

const fn phase_code(phase: RunPhase) -> u8 {
	match phase {
		RunPhase::Init => 1,
		RunPhase::Loop => 2,
		RunPhase::Exit => 3,
	}
}

const fn role_code(role: WorkerTaskRole) -> u8 {
	match role {
		WorkerTaskRole::InitAdmission => 1,
		WorkerTaskRole::Local => 2,
		WorkerTaskRole::ExternalIngress => 3,
		WorkerTaskRole::ExternalEgress => 4,
	}
}

/// Nonblocking completion of one preprovisioned external transfer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExternalTransferPoll {
	Pending,
	Complete { bytes: usize },
}

/// The smallest native hook missing from the whole-bundle [`Backend`] ABI.
///
/// Implementations bind only the immutable worker projection, pre-realize all
/// cross-machine transfer tokens before init, and quiesce every native queue
/// before arena release. Ordinary local tasks, arenas, polling, and teardown
/// continue to use [`Backend`] directly.
pub trait WorkerBackend: Backend {
	type ExternalPending: fmt::Debug;

	fn bind_worker_resources(
		&mut self,
		bundle: &FinalizedBundle,
		projection: &WorkerProjection,
		physical_calls: &mut PhysicalCallBatch,
	) -> Result<Self::Resource, Self::Error>;

	fn prepare_external(
		&mut self,
		resource: &mut Self::Resource,
		transfer: &WorkerExternalTransfer,
		physical_calls: &mut PhysicalCallBatch,
	) -> Result<Self::ExternalPending, Self::Error>;

	fn begin_external_ingress(
		&mut self,
		resource: &mut Self::Resource,
		arenas: ArenaSet<'_, Self::Arena>,
		pending: &mut Self::ExternalPending,
		transfer: &WorkerExternalTransfer,
		bytes: &[u8],
		physical_calls: &mut PhysicalCallBatch,
	) -> Result<(), Self::Error>;

	fn begin_external_egress(
		&mut self,
		resource: &mut Self::Resource,
		arenas: ArenaSet<'_, Self::Arena>,
		pending: &mut Self::ExternalPending,
		transfer: &WorkerExternalTransfer,
		destination: &mut [u8],
		physical_calls: &mut PhysicalCallBatch,
	) -> Result<(), Self::Error>;

	fn poll_external(
		&mut self,
		resource: &mut Self::Resource,
		pending: &mut Self::ExternalPending,
		physical_calls: &mut PhysicalCallBatch,
	) -> Result<ExternalTransferPoll, Self::Error>;

	fn acknowledge_external_egress(
		&mut self,
		resource: &mut Self::Resource,
		pending: &mut Self::ExternalPending,
		transfer: &WorkerExternalTransfer,
		physical_calls: &mut PhysicalCallBatch,
	) -> Result<(), Self::Error>;

	fn quiesce_worker(
		&mut self,
		resource: &mut Self::Resource,
		physical_calls: &mut PhysicalCallBatch,
	) -> Result<(), Self::Error>;
}

/// Physical operation associated with a worker-session failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkerBackendOperation {
	BindProjection,
	PrepareLocal(TaskId),
	PrepareExternal(TaskId),
	AllocateArena(DeviceId),
	SubmitLocal(TaskId),
	PollLocal(TaskId),
	SubmitIngress(TaskId),
	SubmitEgress(TaskId),
	PollExternal(TaskId),
	AcknowledgeEgress(TaskId),
	Quiesce,
	ReleaseArena(DeviceId),
	DestroyResources,
}

/// Current closed lifecycle state of a worker execution session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkerLifecycle {
	Prepared,
	Init,
	Loop,
	Exit,
	Cancelling,
	Finished,
	Failed,
}

/// Nonblocking local task result returned by a worker session.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WorkerTaskPoll {
	Pending,
	Complete { metric: Option<MetricValue> },
}

/// Exact worker-session validation or backend failure.
#[derive(Debug)]
#[non_exhaustive]
pub enum WorkerExecutionError<E: Error + Send + Sync + 'static> {
	Projection(WorkerProjectionError),
	InvalidRun(RunId),
	RunMismatch {
		expected: RunId,
		actual: RunId,
	},
	BundleMismatch,
	UnknownTask(TaskId),
	UnknownDevice(DeviceId),
	WrongRole {
		task: TaskId,
		expected: WorkerTaskRole,
		actual: WorkerTaskRole,
	},
	WrongPhase {
		task: Option<TaskId>,
		expected: RunPhase,
		actual: RunPhase,
	},
	InvalidLifecycle {
		state: WorkerLifecycle,
		detail: &'static str,
	},
	DuplicateDispatch(TaskId),
	TaskNotActive(TaskId),
	DependencyIncomplete {
		task: TaskId,
		dependency: TaskId,
	},
	PhaseIncomplete {
		phase: RunPhase,
		task: TaskId,
	},
	ScheduleConflict {
		task: TaskId,
		active: TaskId,
	},
	ByteCountMismatch {
		task: Option<TaskId>,
		expected: ByteCount,
		actual: ByteCount,
	},
	InitOffsetMismatch {
		device: DeviceId,
		expected: u64,
		actual: u64,
	},
	InitDigestMismatch(DeviceId),
	MetricContract {
		task: TaskId,
		detail: &'static str,
	},
	DeviceFault {
		readback: TaskId,
		code: i32,
	},
	WatchdogExpired {
		task: TaskId,
		polls: u32,
	},
	ArenaAlreadyReleased(DeviceId),
	Backend {
		operation: WorkerBackendOperation,
		source: E,
	},
	Journal(crate::ExecutorError),
	CapacityOverflow,
}

impl<E: Error + Send + Sync + 'static> fmt::Display for WorkerExecutionError<E> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Projection(error) => write!(formatter, "{error}"),
			Self::InvalidRun(run) => write!(formatter, "worker run {run} is invalid"),
			Self::RunMismatch { expected, actual } => {
				write!(
					formatter,
					"worker run {actual} differs from active run {expected}"
				)
			}
			Self::BundleMismatch => formatter.write_str("worker projection and finalized bundle differ"),
			Self::UnknownTask(task) => write!(formatter, "task {task} is outside the worker projection"),
			Self::UnknownDevice(device) => {
				write!(
					formatter,
					"device {device} is outside the worker projection"
				)
			}
			Self::WrongRole {
				task,
				expected,
				actual,
			} => {
				write!(
					formatter,
					"worker task {task} has role {actual:?}, expected {expected:?}"
				)
			}
			Self::WrongPhase {
				task,
				expected,
				actual,
			} => {
				write!(
					formatter,
					"worker task {task:?} belongs to {actual:?}, active phase is {expected:?}"
				)
			}
			Self::InvalidLifecycle { state, detail } => {
				write!(
					formatter,
					"worker lifecycle {state:?} rejected operation: {detail}"
				)
			}
			Self::DuplicateDispatch(task) => {
				write!(
					formatter,
					"worker task {task} was dispatched more than once"
				)
			}
			Self::TaskNotActive(task) => write!(formatter, "worker task {task} is not active"),
			Self::DependencyIncomplete { task, dependency } => {
				write!(
					formatter,
					"worker task {task} depends on incomplete task {dependency}"
				)
			}
			Self::PhaseIncomplete { phase, task } => {
				write!(
					formatter,
					"worker {phase:?} phase still has incomplete task {task}"
				)
			}
			Self::ScheduleConflict { task, active } => {
				write!(
					formatter,
					"worker task {task} conflicts with active schedule window for {active}"
				)
			}
			Self::ByteCountMismatch {
				task,
				expected,
				actual,
			} => {
				write!(
					formatter,
					"worker task {task:?} received {actual} bytes, expected {expected}"
				)
			}
			Self::InitOffsetMismatch {
				device,
				expected,
				actual,
			} => {
				write!(
					formatter,
					"worker device {device} init offset {actual} differs from expected {expected}"
				)
			}
			Self::InitDigestMismatch(device) => {
				write!(formatter, "worker device {device} init digest differs")
			}
			Self::MetricContract { task, detail } => {
				write!(
					formatter,
					"worker metric task {task} violated its contract: {detail}"
				)
			}
			Self::DeviceFault { readback, code } => {
				write!(
					formatter,
					"worker checked device work reported fault {code} through task {readback}"
				)
			}
			Self::WatchdogExpired { task, polls } => {
				write!(
					formatter,
					"worker task {task} made no progress for {polls} polls"
				)
			}
			Self::ArenaAlreadyReleased(device) => write!(formatter, "worker arena {device} was already released"),
			Self::Backend { operation, source } => {
				write!(formatter, "worker backend {operation:?} failed: {source}")
			}
			Self::Journal(error) => write!(formatter, "worker accounting failed: {error}"),
			Self::CapacityOverflow => formatter.write_str("worker session capacity arithmetic overflowed"),
		}
	}
}

impl<E: Error + Send + Sync + 'static> Error for WorkerExecutionError<E> {
	fn source(&self) -> Option<&(dyn Error + 'static)> {
		match self {
			Self::Projection(error) => Some(error),
			Self::Backend { source, .. } => Some(source),
			Self::Journal(error) => Some(error),
			_ => None,
		}
	}
}

/// Failed pre-init preparation with backend ownership preserved.
#[derive(Debug)]
pub struct WorkerPrepareFailure<B: WorkerBackend> {
	error: Box<WorkerExecutionError<B::Error>>,
	cleanup_error: Option<Box<WorkerExecutionError<B::Error>>>,
	backend: B,
}

impl<B: WorkerBackend> WorkerPrepareFailure<B> {
	#[must_use]
	pub fn error(&self) -> &WorkerExecutionError<B::Error> { &self.error }

	#[must_use]
	pub fn cleanup_error(&self) -> Option<&WorkerExecutionError<B::Error>> { self.cleanup_error.as_deref() }

	pub fn into_parts(self) -> WorkerPrepareFailureParts<B> {
		WorkerPrepareFailureParts {
			error: *self.error,
			cleanup_error: self.cleanup_error.map(|error| *error),
			backend: self.backend,
		}
	}
}

/// Owned preparation failure parts, including the recovered backend.
#[derive(Debug)]
pub struct WorkerPrepareFailureParts<B: WorkerBackend> {
	pub error: WorkerExecutionError<B::Error>,
	pub cleanup_error: Option<WorkerExecutionError<B::Error>>,
	pub backend: B,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TaskState {
	Idle,
	Active,
	AwaitingAck,
	Complete,
	Quiesced,
}

#[derive(Debug)]
enum WorkerPending<P, E> {
	Local(P),
	External(E),
}

#[derive(Debug)]
struct WorkerTaskSlot<P, E> {
	contract: ProjectedTask,
	pending: WorkerPending<P, E>,
	state: TaskState,
	nonprogress_polls: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ImageState {
	Needed,
	Receiving,
	Submitted,
	Complete,
	Released,
}

#[derive(Debug)]
struct WorkerImage {
	device: DeviceId,
	task: TaskId,
	bytes: ByteCount,
	expected_digest: Digest,
	received: u64,
	buffer: Box<[u8]>,
	state: ImageState,
}

/// Prepared per-worker execution over one exact immutable projection.
#[derive(Debug)]
pub struct WorkerExecutionSession<B: WorkerBackend> {
	run: RunId,
	loop_iteration: LoopIteration,
	projection: WorkerProjection,
	backend: B,
	resource: Option<B::Resource>,
	arenas: BTreeMap<DeviceId, B::Arena>,
	tasks: Box<[WorkerTaskSlot<B::Pending, B::ExternalPending>]>,
	images: Box<[WorkerImage]>,
	lifecycle: WorkerLifecycle,
	watchdog: Watchdog,
	journal: RunJournal,
}

impl<B: WorkerBackend> WorkerExecutionSession<B> {
	pub fn prepare(
		run: RunId,
		bundle: &FinalizedBundle,
		projection: WorkerProjection,
		mut backend: B,
		watchdog: Watchdog,
	) -> Result<Self, WorkerPrepareFailure<B>> {
		if run.get() == 0 {
			return Err(WorkerPrepareFailure {
				error: Box::new(WorkerExecutionError::InvalidRun(run)),
				cleanup_error: None,
				backend,
			});
		}
		if projection.bundle != bundle.identity() || projection.topology != bundle.topology() {
			return Err(WorkerPrepareFailure {
				error: Box::new(WorkerExecutionError::BundleMismatch),
				cleanup_error: None,
				backend,
			});
		}
		let journal_capacity = match worker_journal_capacity::<B>(bundle, &projection) {
			Ok(capacity) => capacity,
			Err(error) => {
				return Err(WorkerPrepareFailure {
					error: Box::new(error),
					cleanup_error: None,
					backend,
				});
			}
		};
		let mut journal = RunJournal::with_capacity(journal_capacity, bundle.tasks());
		let mut calls = PhysicalCallBatch::new();
		let resource = match backend.bind_worker_resources(bundle, &projection, &mut calls) {
			Ok(resource) => resource,
			Err(source) => {
				let journal_error = journal
					.record_physical(calls)
					.err()
					.map(WorkerExecutionError::Journal)
					.map(Box::new);
				return Err(WorkerPrepareFailure {
					error: Box::new(WorkerExecutionError::Backend {
						operation: WorkerBackendOperation::BindProjection,
						source,
					}),
					cleanup_error: journal_error,
					backend,
				});
			}
		};
		if let Err(error) = journal.record_physical(calls) {
			let cleanup_error = destroy_prepared_resource(&mut backend, resource, &mut journal);
			return Err(WorkerPrepareFailure {
				error: Box::new(WorkerExecutionError::Journal(error)),
				cleanup_error: cleanup_error.map(Box::new),
				backend,
			});
		}
		let mut resource = resource;
		let mut slots = Vec::with_capacity(projection.tasks.len());
		for contract in projection.tasks.iter().cloned() {
			let mut calls = PhysicalCallBatch::new();
			let pending = match &contract.work {
				PreparedWorkerWork::External(transfer) => {
					backend
						.prepare_external(&mut resource, transfer, &mut calls)
						.map(WorkerPending::External)
						.map_err(|source| {
							WorkerExecutionError::Backend {
								operation: WorkerBackendOperation::PrepareExternal(contract.id),
								source,
							}
						})
				}
				work => {
					let request = PendingRequest {
						task: contract.id,
						phase: contract.phase,
						class: match work.class() {
							Some(class) => class,
							None => {
								return Err(preparation_failure(
									backend,
									resource,
									journal,
									WorkerExecutionError::Projection(
										WorkerProjectionError::InvalidTask {
											task: contract.id,
											detail: "local task has no backend work class",
										},
									),
								));
							}
						},
						submission: work.submission(),
					};
					backend
						.prepare_pending(&mut resource, request, &mut calls)
						.map(WorkerPending::Local)
						.map_err(|source| {
							WorkerExecutionError::Backend {
								operation: WorkerBackendOperation::PrepareLocal(contract.id),
								source,
							}
						})
				}
			};
			let pending = match pending {
				Ok(pending) => pending,
				Err(error) => {
					let record_error = journal
						.record_physical(calls)
						.err()
						.map(WorkerExecutionError::Journal)
						.map(Box::new);
					let mut failure = preparation_failure(backend, resource, journal, error);
					if failure.cleanup_error.is_none() {
						failure.cleanup_error = record_error;
					}
					return Err(failure);
				}
			};
			if let Err(error) = journal.record_physical(calls) {
				return Err(preparation_failure(
					backend,
					resource,
					journal,
					WorkerExecutionError::Journal(error),
				));
			}
			slots.push(WorkerTaskSlot {
				contract,
				pending,
				state: TaskState::Idle,
				nonprogress_polls: 0,
			});
		}
		let images = match prepare_images::<B>(&projection) {
			Ok(images) => images,
			Err(error) => {
				return Err(preparation_failure(backend, resource, journal, error));
			}
		};
		if let Err(error) = journal.record_logical(LogicalEvent::Prepared {
			run,
			bundle: bundle.identity(),
		}) {
			return Err(preparation_failure(
				backend,
				resource,
				journal,
				WorkerExecutionError::Journal(error),
			));
		}
		Ok(Self {
			run,
			loop_iteration: bundle
				.loop_iterations()
				.iteration(0)
				.expect("a finalized loop always contains iteration zero"),
			projection,
			backend,
			resource: Some(resource),
			arenas: BTreeMap::new(),
			tasks: slots.into_boxed_slice(),
			images,
			lifecycle: WorkerLifecycle::Prepared,
			watchdog,
			journal,
		})
	}

	#[must_use]
	pub const fn run_id(&self) -> RunId { self.run }

	#[must_use]
	pub const fn projection(&self) -> &WorkerProjection { &self.projection }

	#[must_use]
	pub const fn lifecycle(&self) -> WorkerLifecycle { self.lifecycle }

	#[must_use]
	pub const fn journal(&self) -> &RunJournal { &self.journal }

	pub fn begin_init_image(
		&mut self,
		run: RunId,
		device: DeviceId,
		bytes: ByteCount,
		digest: Digest,
	) -> Result<(), WorkerExecutionError<B::Error>> {
		self.ensure_run(run)?;
		if !matches!(
			self.lifecycle,
			WorkerLifecycle::Prepared | WorkerLifecycle::Init
		) {
			return Err(self.lifecycle_error("init image cannot begin in this state"));
		}
		let image_index = self.image_index(device)?;
		let image_task = self.images[image_index].task;
		let image_bytes = self.images[image_index].bytes;
		if self.images[image_index].state != ImageState::Needed {
			return Err(WorkerExecutionError::DuplicateDispatch(image_task));
		}
		if image_bytes != bytes {
			return Err(WorkerExecutionError::ByteCountMismatch {
				task: Some(image_task),
				expected: image_bytes,
				actual: bytes,
			});
		}
		let layout = self
			.projection
			.layouts
			.iter()
			.find(|layout| layout.device == device)
			.ok_or(WorkerExecutionError::UnknownDevice(device))?
			.clone();
		let arena_bytes = layout.size;
		if self.arenas.contains_key(&device) {
			return Err(WorkerExecutionError::DuplicateDispatch(image_task));
		}
		let mut calls = PhysicalCallBatch::new();
		let result = {
			let resource = self
				.resource
				.as_mut()
				.ok_or(WorkerExecutionError::InvalidLifecycle {
					state: self.lifecycle,
					detail: "worker resources are unavailable",
				})?;
			self.backend.allocate_arena(resource, &layout, &mut calls)
		};
		let arena = self.backend_result(WorkerBackendOperation::AllocateArena(device), calls, result)?;
		let replaced = self.arenas.insert(device, arena);
		debug_assert!(replaced.is_none());
		let image = &mut self.images[image_index];
		image.expected_digest = digest;
		image.received = 0;
		image.buffer.fill(0);
		image.state = ImageState::Receiving;
		self.lifecycle = WorkerLifecycle::Init;
		Self::journal_result(self.journal.record_logical(LogicalEvent::ArenaAllocated {
			device,
			bytes: arena_bytes,
		}))
	}

	pub fn write_init_chunk(
		&mut self,
		run: RunId,
		device: DeviceId,
		offset: u64,
		bytes: &[u8],
	) -> Result<usize, WorkerExecutionError<B::Error>> {
		self.ensure_run(run)?;
		let index = self.image_index(device)?;
		let image = &mut self.images[index];
		if image.state != ImageState::Receiving {
			return Err(self.lifecycle_error("init chunk has no receiving image"));
		}
		if image.received != offset {
			return Err(WorkerExecutionError::InitOffsetMismatch {
				device,
				expected: image.received,
				actual: offset,
			});
		}
		let length = u64::try_from(bytes.len()).map_err(|_| WorkerExecutionError::CapacityOverflow)?;
		let end = offset
			.checked_add(length)
			.ok_or(WorkerExecutionError::CapacityOverflow)?;
		if end > image.bytes.get() {
			return Err(WorkerExecutionError::ByteCountMismatch {
				task: Some(image.task),
				expected: image.bytes,
				actual: ByteCount::new(end),
			});
		}
		let start = usize::try_from(offset).map_err(|_| WorkerExecutionError::CapacityOverflow)?;
		let end_index = start
			.checked_add(bytes.len())
			.ok_or(WorkerExecutionError::CapacityOverflow)?;
		image.buffer[start..end_index].copy_from_slice(bytes);
		image.received = end;
		Ok(bytes.len())
	}

	pub fn submit_init_image(&mut self, run: RunId, device: DeviceId) -> Result<(), WorkerExecutionError<B::Error>> {
		self.ensure_run(run)?;
		let image_index = self.image_index(device)?;
		let task_id = self.images[image_index].task;
		if self.images[image_index].state != ImageState::Receiving
			|| self.images[image_index].received != self.images[image_index].bytes.get()
		{
			return Err(self.lifecycle_error("init image is incomplete"));
		}
		let actual = Digest::new(Sha256::digest(&self.images[image_index].buffer).into());
		if actual != self.images[image_index].expected_digest {
			return Err(WorkerExecutionError::InitDigestMismatch(device));
		}
		let task_index = self.task_index(task_id)?;
		self.ensure_dependencies(task_index)?;
		let mut calls = PhysicalCallBatch::new();
		let result = {
			let resource = self
				.resource
				.as_mut()
				.ok_or(WorkerExecutionError::InvalidLifecycle {
					state: self.lifecycle,
					detail: "worker resources are unavailable",
				})?;
			let slot = &mut self.tasks[task_index];
			let role = slot.contract.role();
			let work = slot
				.contract
				.work
				.backend_work(task_id, run, None, Some(&self.images[image_index].buffer))
				.ok_or(WorkerExecutionError::WrongRole {
					task: task_id,
					expected: WorkerTaskRole::InitAdmission,
					actual: role,
				})?;
			let WorkerPending::Local(pending) = &mut slot.pending else {
				return Err(WorkerExecutionError::WrongRole {
					task: task_id,
					expected: WorkerTaskRole::InitAdmission,
					actual: role,
				});
			};
			self.backend.submit(
				resource,
				ArenaSet::new(&self.arenas),
				pending,
				work,
				&mut calls,
			)
		};
		self.backend_result(WorkerBackendOperation::SubmitLocal(task_id), calls, result)?;
		self.tasks[task_index].state = TaskState::Active;
		self.images[image_index].state = ImageState::Submitted;
		Self::journal_result(self.journal.record_logical(LogicalEvent::InitAdmission {
			task: task_id,
			device,
			bytes: self.images[image_index].bytes,
		}))
	}

	pub fn poll_init_image(
		&mut self,
		run: RunId,
		device: DeviceId,
	) -> Result<ExternalTransferPoll, WorkerExecutionError<B::Error>> {
		self.ensure_run(run)?;
		let image_index = self.image_index(device)?;
		let task = self.images[image_index].task;
		if self.images[image_index].state != ImageState::Submitted {
			return Err(WorkerExecutionError::TaskNotActive(task));
		}
		let task_index = self.task_index(task)?;
		let poll = self.poll_local_pending(task_index)?;
		match poll {
			BackendPoll::Pending => Ok(ExternalTransferPoll::Pending),
			BackendPoll::Complete { metric: None } => {
				self.tasks[task_index].state = TaskState::Complete;
				self.images[image_index].state = ImageState::Complete;
				Self::journal_result(self.journal.record_logical(LogicalEvent::TaskCompleted {
					phase: RunPhase::Init,
					task,
				}))?;
				if self
					.images
					.iter()
					.all(|image| image.state == ImageState::Complete)
				{
					self.lifecycle = WorkerLifecycle::Loop;
					Self::journal_result(
						self.journal
							.record_logical(LogicalEvent::Initialized { run }),
					)?;
					Self::journal_result(
						self.journal
							.record_logical(LogicalEvent::LoopStarted { run }),
					)?;
				}
				Ok(ExternalTransferPoll::Complete {
					bytes: usize::try_from(self.images[image_index].bytes.get())
						.map_err(|_| WorkerExecutionError::CapacityOverflow)?,
				})
			}
			BackendPoll::Complete { metric: Some(_) } => {
				Err(WorkerExecutionError::MetricContract {
					task,
					detail: "init admission completed with a metric",
				})
			}
		}
	}

	pub fn submit_task(
		&mut self,
		run: RunId,
		phase: RunPhase,
		task: TaskId,
	) -> Result<(), WorkerExecutionError<B::Error>> {
		let index = self.ensure_dispatch(run, phase, task, WorkerTaskRole::Local)?;
		let class = self.tasks[index]
			.contract
			.work
			.class()
			.ok_or(WorkerExecutionError::WrongRole {
				task,
				expected: WorkerTaskRole::Local,
				actual: self.tasks[index].contract.role(),
			})?;
		let mut calls = PhysicalCallBatch::new();
		let result = {
			let resource = self
				.resource
				.as_mut()
				.ok_or(WorkerExecutionError::InvalidLifecycle {
					state: self.lifecycle,
					detail: "worker resources are unavailable",
				})?;
			let slot = &mut self.tasks[index];
			let role = slot.contract.role();
			let work = slot
				.contract
				.work
				.backend_work(task, run, Some(self.loop_iteration), None)
				.ok_or(WorkerExecutionError::WrongRole {
					task,
					expected: WorkerTaskRole::Local,
					actual: role,
				})?;
			let WorkerPending::Local(pending) = &mut slot.pending else {
				return Err(WorkerExecutionError::WrongRole {
					task,
					expected: WorkerTaskRole::Local,
					actual: role,
				});
			};
			self.backend.submit(
				resource,
				ArenaSet::new(&self.arenas),
				pending,
				work,
				&mut calls,
			)
		};
		self.backend_result(WorkerBackendOperation::SubmitLocal(task), calls, result)?;
		self.tasks[index].state = TaskState::Active;
		Self::journal_result(
			self.journal
				.record_logical(LogicalEvent::TaskSubmitted { phase, task, class }),
		)
	}

	pub fn poll_task(&mut self, run: RunId, task: TaskId) -> Result<WorkerTaskPoll, WorkerExecutionError<B::Error>> {
		self.ensure_run(run)?;
		let index = self.task_index(task)?;
		if self.tasks[index].contract.role() != WorkerTaskRole::Local {
			return Err(WorkerExecutionError::WrongRole {
				task,
				expected: WorkerTaskRole::Local,
				actual: self.tasks[index].contract.role(),
			});
		}
		if self.tasks[index].state != TaskState::Active {
			return Err(WorkerExecutionError::TaskNotActive(task));
		}
		match self.poll_local_pending(index)? {
			BackendPoll::Pending => Ok(WorkerTaskPoll::Pending),
			BackendPoll::Complete { metric } => {
				let metric = self.validate_metric_completion(index, metric)?;
				self.tasks[index].state = TaskState::Complete;
				Self::journal_result(self.journal.record_logical(LogicalEvent::TaskCompleted {
					phase: self.tasks[index].contract.phase,
					task,
				}))?;
				Ok(WorkerTaskPoll::Complete { metric })
			}
		}
	}

	pub fn begin_external_ingress(
		&mut self,
		run: RunId,
		phase: RunPhase,
		task: TaskId,
		bytes: &[u8],
	) -> Result<(), WorkerExecutionError<B::Error>> {
		let index = self.ensure_dispatch(run, phase, task, WorkerTaskRole::ExternalIngress)?;
		let transfer = self.external_contract(index)?;
		let actual = byte_count(bytes.len())?;
		if actual != transfer.bytes {
			return Err(WorkerExecutionError::ByteCountMismatch {
				task: Some(task),
				expected: transfer.bytes,
				actual,
			});
		}
		let transfer_bytes = transfer.bytes;
		let mut calls = PhysicalCallBatch::new();
		let result = {
			let resource = self
				.resource
				.as_mut()
				.ok_or(WorkerExecutionError::InvalidLifecycle {
					state: self.lifecycle,
					detail: "worker resources are unavailable",
				})?;
			let slot = &mut self.tasks[index];
			let role = slot.contract.role();
			let PreparedWorkerWork::External(transfer) = &slot.contract.work else {
				return Err(WorkerExecutionError::WrongRole {
					task,
					expected: WorkerTaskRole::ExternalIngress,
					actual: role,
				});
			};
			let WorkerPending::External(pending) = &mut slot.pending else {
				return Err(WorkerExecutionError::WrongRole {
					task,
					expected: WorkerTaskRole::ExternalIngress,
					actual: role,
				});
			};
			self.backend.begin_external_ingress(
				resource,
				ArenaSet::new(&self.arenas),
				pending,
				transfer,
				bytes,
				&mut calls,
			)
		};
		self.backend_result(WorkerBackendOperation::SubmitIngress(task), calls, result)?;
		self.tasks[index].state = TaskState::Active;
		Self::journal_result(
			self.journal
				.record_logical(LogicalEvent::ExternalTransferSubmitted {
					phase,
					task,
					direction: ExternalTransferDirection::Ingress,
					bytes: transfer_bytes,
				}),
		)
	}

	pub fn poll_external_ingress(
		&mut self,
		run: RunId,
		task: TaskId,
	) -> Result<ExternalTransferPoll, WorkerExecutionError<B::Error>> {
		self.poll_external_direction(run, task, ExternalTransferDirection::Ingress)
	}

	pub fn begin_external_egress(
		&mut self,
		run: RunId,
		phase: RunPhase,
		task: TaskId,
		destination: &mut [u8],
	) -> Result<(), WorkerExecutionError<B::Error>> {
		let index = self.ensure_dispatch(run, phase, task, WorkerTaskRole::ExternalEgress)?;
		let transfer = self.external_contract(index)?;
		let actual = byte_count(destination.len())?;
		if actual != transfer.bytes {
			return Err(WorkerExecutionError::ByteCountMismatch {
				task: Some(task),
				expected: transfer.bytes,
				actual,
			});
		}
		let transfer_bytes = transfer.bytes;
		let mut calls = PhysicalCallBatch::new();
		let result = {
			let resource = self
				.resource
				.as_mut()
				.ok_or(WorkerExecutionError::InvalidLifecycle {
					state: self.lifecycle,
					detail: "worker resources are unavailable",
				})?;
			let slot = &mut self.tasks[index];
			let role = slot.contract.role();
			let PreparedWorkerWork::External(transfer) = &slot.contract.work else {
				return Err(WorkerExecutionError::WrongRole {
					task,
					expected: WorkerTaskRole::ExternalEgress,
					actual: role,
				});
			};
			let WorkerPending::External(pending) = &mut slot.pending else {
				return Err(WorkerExecutionError::WrongRole {
					task,
					expected: WorkerTaskRole::ExternalEgress,
					actual: role,
				});
			};
			self.backend.begin_external_egress(
				resource,
				ArenaSet::new(&self.arenas),
				pending,
				transfer,
				destination,
				&mut calls,
			)
		};
		self.backend_result(WorkerBackendOperation::SubmitEgress(task), calls, result)?;
		self.tasks[index].state = TaskState::Active;
		Self::journal_result(
			self.journal
				.record_logical(LogicalEvent::ExternalTransferSubmitted {
					phase,
					task,
					direction: ExternalTransferDirection::Egress,
					bytes: transfer_bytes,
				}),
		)
	}

	pub fn poll_external_egress(
		&mut self,
		run: RunId,
		task: TaskId,
	) -> Result<ExternalTransferPoll, WorkerExecutionError<B::Error>> {
		self.poll_external_direction(run, task, ExternalTransferDirection::Egress)
	}

	pub fn acknowledge_external_egress(
		&mut self,
		run: RunId,
		task: TaskId,
	) -> Result<(), WorkerExecutionError<B::Error>> {
		self.ensure_run(run)?;
		let index = self.task_index(task)?;
		let transfer = self.external_contract(index)?;
		if transfer.direction != ExternalTransferDirection::Egress {
			return Err(WorkerExecutionError::WrongRole {
				task,
				expected: WorkerTaskRole::ExternalEgress,
				actual: self.tasks[index].contract.role(),
			});
		}
		if self.tasks[index].state != TaskState::AwaitingAck {
			return Err(WorkerExecutionError::TaskNotActive(task));
		}
		let transfer_phase = transfer.phase;
		let transfer_direction = transfer.direction;
		let transfer_bytes = transfer.bytes;
		let mut calls = PhysicalCallBatch::new();
		let result = {
			let resource = self
				.resource
				.as_mut()
				.ok_or(WorkerExecutionError::InvalidLifecycle {
					state: self.lifecycle,
					detail: "worker resources are unavailable",
				})?;
			let slot = &mut self.tasks[index];
			let role = slot.contract.role();
			let PreparedWorkerWork::External(transfer) = &slot.contract.work else {
				return Err(WorkerExecutionError::WrongRole {
					task,
					expected: WorkerTaskRole::ExternalEgress,
					actual: role,
				});
			};
			let WorkerPending::External(pending) = &mut slot.pending else {
				return Err(WorkerExecutionError::WrongRole {
					task,
					expected: WorkerTaskRole::ExternalEgress,
					actual: role,
				});
			};
			self.backend
				.acknowledge_external_egress(resource, pending, transfer, &mut calls)
		};
		self.backend_result(
			WorkerBackendOperation::AcknowledgeEgress(task),
			calls,
			result,
		)?;
		self.tasks[index].state = TaskState::Complete;
		Self::journal_result(
			self.journal
				.record_logical(LogicalEvent::ExternalTransferCompleted {
					phase: transfer_phase,
					task,
					direction: transfer_direction,
					bytes: transfer_bytes,
				}),
		)
	}

	pub fn begin_exit(&mut self, run: RunId) -> Result<(), WorkerExecutionError<B::Error>> {
		self.ensure_run(run)?;
		if self.lifecycle != WorkerLifecycle::Loop {
			return Err(self.lifecycle_error("exit may begin only after worker loop entry"));
		}
		if let Some(task) = self
			.tasks
			.iter()
			.find(|slot| slot.contract.phase == RunPhase::Loop && slot.state != TaskState::Complete)
		{
			return Err(WorkerExecutionError::PhaseIncomplete {
				phase: RunPhase::Loop,
				task: task.contract.id,
			});
		}
		self.lifecycle = WorkerLifecycle::Exit;
		Self::journal_result(
			self.journal
				.record_logical(LogicalEvent::LoopCompleted { run }),
		)
	}

	pub fn cancel(&mut self, run: RunId) -> Result<(), WorkerExecutionError<B::Error>> {
		self.ensure_run(run)?;
		if !matches!(
			self.lifecycle,
			WorkerLifecycle::Loop | WorkerLifecycle::Exit
		) {
			return Err(self.lifecycle_error("cancellation requires an active loop or exit"));
		}
		self.quiesce()?;
		self.lifecycle = WorkerLifecycle::Cancelling;
		Ok(())
	}

	pub fn release_arena(&mut self, run: RunId, device: DeviceId) -> Result<(), WorkerExecutionError<B::Error>> {
		self.ensure_run(run)?;
		match self.lifecycle {
			WorkerLifecycle::Exit => {
				if self
					.tasks
					.iter()
					.any(|slot| slot.contract.phase == RunPhase::Exit && slot.state != TaskState::Complete)
				{
					return Err(self.lifecycle_error("exit arena release precedes projected exit completion"));
				}
			}
			WorkerLifecycle::Cancelling => {}
			state => {
				return Err(WorkerExecutionError::InvalidLifecycle {
					state,
					detail: "arena release requires completed exit or cancellation",
				});
			}
		}
		self.release_one_arena(device)
	}

	pub fn finish(&mut self, run: RunId) -> Result<(), WorkerExecutionError<B::Error>> {
		self.ensure_run(run)?;
		if !matches!(
			self.lifecycle,
			WorkerLifecycle::Exit | WorkerLifecycle::Cancelling
		) {
			return Err(self.lifecycle_error("finish requires exit or cancellation"));
		}
		if !self.arenas.is_empty() {
			return Err(self.lifecycle_error("finish requires exact release of every worker arena"));
		}
		let resource = self
			.resource
			.take()
			.ok_or_else(|| self.lifecycle_error("worker resources were already destroyed"))?;
		let mut calls = PhysicalCallBatch::new();
		let result = self.backend.destroy_resources(resource, &mut calls);
		self.backend_result(WorkerBackendOperation::DestroyResources, calls, result)?;
		self.lifecycle = WorkerLifecycle::Finished;
		Self::journal_result(self.journal.record_logical(LogicalEvent::Exited { run }))
	}

	/// Quiesces all native work before releasing each extant arena once and
	/// destroying resources. A failed quiesce deliberately preserves every
	/// arena because native work may still reference it.
	pub fn fatal_cleanup(&mut self) -> Result<(), WorkerExecutionError<B::Error>> {
		let mut first_error = None;
		if self.resource.is_some() {
			let mut calls = PhysicalCallBatch::new();
			let result = {
				let resource = self
					.resource
					.as_mut()
					.ok_or(WorkerExecutionError::InvalidLifecycle {
						state: self.lifecycle,
						detail: "worker resources are unavailable",
					})?;
				self.backend.quiesce_worker(resource, &mut calls)
			};
			let record = self.journal.record_physical(calls);
			match result {
				Ok(()) => {
					for slot in &mut self.tasks {
						if matches!(slot.state, TaskState::Active | TaskState::AwaitingAck) {
							slot.state = TaskState::Quiesced;
						}
					}
				}
				Err(source) => {
					self.lifecycle = WorkerLifecycle::Failed;
					return Err(WorkerExecutionError::Backend {
						operation: WorkerBackendOperation::Quiesce,
						source,
					});
				}
			}
			if let Err(error) = record {
				first_error = Some(WorkerExecutionError::Journal(error));
			}
			if let Err(error) = self
				.journal
				.record_logical(LogicalEvent::WorkerQuiesced { run: self.run })
				&& first_error.is_none()
			{
				first_error = Some(WorkerExecutionError::Journal(error));
			}
		}
		while let Some(device) = self.arenas.keys().next().copied() {
			if let Err(error) = self.release_one_arena(device)
				&& first_error.is_none()
			{
				first_error = Some(error);
			}
		}
		for image in &mut self.images {
			image.buffer.fill(0);
		}
		if let Some(resource) = self.resource.take() {
			let mut calls = PhysicalCallBatch::new();
			let result = self.backend.destroy_resources(resource, &mut calls);
			if let Err(error) = self.backend_result(WorkerBackendOperation::DestroyResources, calls, result)
				&& first_error.is_none()
			{
				first_error = Some(error);
			}
		}
		self.lifecycle = WorkerLifecycle::Failed;
		match first_error {
			Some(error) => Err(error),
			None => Ok(()),
		}
	}

	pub fn into_parts(self) -> Result<(B, RunJournal), Box<Self>> {
		match self.lifecycle {
			WorkerLifecycle::Finished | WorkerLifecycle::Failed if self.resource.is_none() => {
				Ok((self.backend, self.journal))
			}
			_ => Err(Box::new(self)),
		}
	}

	fn poll_external_direction(
		&mut self,
		run: RunId,
		task: TaskId,
		direction: ExternalTransferDirection,
	) -> Result<ExternalTransferPoll, WorkerExecutionError<B::Error>> {
		self.ensure_run(run)?;
		let index = self.task_index(task)?;
		let (contract_direction, transfer_phase, transfer_bytes) = {
			let transfer = self.external_contract(index)?;
			(transfer.direction, transfer.phase, transfer.bytes)
		};
		if contract_direction != direction {
			return Err(WorkerExecutionError::WrongRole {
				task,
				expected: match direction {
					ExternalTransferDirection::Ingress => WorkerTaskRole::ExternalIngress,
					ExternalTransferDirection::Egress => WorkerTaskRole::ExternalEgress,
				},
				actual: self.tasks[index].contract.role(),
			});
		}
		if self.tasks[index].state != TaskState::Active {
			return Err(WorkerExecutionError::TaskNotActive(task));
		}
		let mut calls = PhysicalCallBatch::new();
		let result = {
			let resource = self
				.resource
				.as_mut()
				.ok_or(WorkerExecutionError::InvalidLifecycle {
					state: self.lifecycle,
					detail: "worker resources are unavailable",
				})?;
			let WorkerPending::External(pending) = &mut self.tasks[index].pending else {
				return Err(WorkerExecutionError::TaskNotActive(task));
			};
			self.backend.poll_external(resource, pending, &mut calls)
		};
		let poll = self.backend_result(WorkerBackendOperation::PollExternal(task), calls, result)?;
		match poll {
			ExternalTransferPoll::Pending => {
				self.bump_watchdog(index)?;
				Ok(ExternalTransferPoll::Pending)
			}
			ExternalTransferPoll::Complete { bytes } => {
				self.tasks[index].nonprogress_polls = 0;
				let actual = byte_count(bytes)?;
				if actual != transfer_bytes {
					return Err(WorkerExecutionError::ByteCountMismatch {
						task: Some(task),
						expected: transfer_bytes,
						actual,
					});
				}
				match direction {
					ExternalTransferDirection::Ingress => {
						self.tasks[index].state = TaskState::Complete;
						Self::journal_result(self.journal.record_logical(
							LogicalEvent::ExternalTransferCompleted {
								phase: transfer_phase,
								task,
								direction,
								bytes: transfer_bytes,
							},
						))?;
					}
					ExternalTransferDirection::Egress => {
						self.tasks[index].state = TaskState::AwaitingAck;
					}
				}
				Ok(ExternalTransferPoll::Complete { bytes })
			}
		}
	}

	fn poll_local_pending(&mut self, index: usize) -> Result<BackendPoll, WorkerExecutionError<B::Error>> {
		let task = self.tasks[index].contract.id;
		let mut calls = PhysicalCallBatch::new();
		let result = {
			let resource = self
				.resource
				.as_mut()
				.ok_or(WorkerExecutionError::InvalidLifecycle {
					state: self.lifecycle,
					detail: "worker resources are unavailable",
				})?;
			let WorkerPending::Local(pending) = &mut self.tasks[index].pending else {
				return Err(WorkerExecutionError::TaskNotActive(task));
			};
			self.backend.poll(resource, pending, &mut calls)
		};
		let poll = self.backend_result(WorkerBackendOperation::PollLocal(task), calls, result)?;
		match poll {
			BackendPoll::Pending => self.bump_watchdog(index)?,
			BackendPoll::Complete { .. } => self.tasks[index].nonprogress_polls = 0,
		}
		Ok(poll)
	}

	fn validate_metric_completion(
		&self,
		index: usize,
		metric: Option<MetricValue>,
	) -> Result<Option<MetricValue>, WorkerExecutionError<B::Error>> {
		let task = &self.tasks[index].contract;
		match (&task.work, metric) {
			(
				PreparedWorkerWork::Metric {
					purpose: MetricPurpose::User,
					value,
					..
				},
				Some(metric),
			) => {
				let dtype_matches = matches!(
					(value.dtype, metric),
					(DType::F32, MetricValue::F32(_)) | (DType::I32, MetricValue::I32(_))
				);
				match dtype_matches {
					true => Ok(Some(metric)),
					false => {
						Err(WorkerExecutionError::MetricContract {
							task: task.id,
							detail: "user metric dtype differs from its finalized value",
						})
					}
				}
			}
			(
				PreparedWorkerWork::Metric {
					purpose: MetricPurpose::FaultReadback,
					..
				},
				Some(MetricValue::I32(0)),
			) => Ok(None),
			(
				PreparedWorkerWork::Metric {
					purpose: MetricPurpose::FaultReadback,
					..
				},
				Some(MetricValue::I32(code)),
			) => {
				Err(WorkerExecutionError::DeviceFault {
					readback: task.id,
					code,
				})
			}
			(PreparedWorkerWork::Metric { .. }, Some(MetricValue::F32(_)) | None) => {
				Err(WorkerExecutionError::MetricContract {
					task: task.id,
					detail: "metric completion is absent or incompatible",
				})
			}
			(PreparedWorkerWork::InitAdmission { .. }, None)
			| (PreparedWorkerWork::Calculation { .. }, None)
			| (PreparedWorkerWork::InternalTransfer { .. }, None) => Ok(None),
			(PreparedWorkerWork::External(_), _) => {
				Err(WorkerExecutionError::WrongRole {
					task: task.id,
					expected: WorkerTaskRole::Local,
					actual: task.role(),
				})
			}
			(_, Some(_)) => {
				Err(WorkerExecutionError::MetricContract {
					task: task.id,
					detail: "non-metric task completed with a metric",
				})
			}
		}
	}

	fn ensure_dispatch(
		&self,
		run: RunId,
		phase: RunPhase,
		task: TaskId,
		role: WorkerTaskRole,
	) -> Result<usize, WorkerExecutionError<B::Error>> {
		self.ensure_run(run)?;
		let active_phase = self.active_phase()?;
		if active_phase != phase {
			return Err(WorkerExecutionError::WrongPhase {
				task: Some(task),
				expected: active_phase,
				actual: phase,
			});
		}
		let index = self.task_index(task)?;
		let contract = &self.tasks[index].contract;
		if contract.phase != phase {
			return Err(WorkerExecutionError::WrongPhase {
				task: Some(task),
				expected: phase,
				actual: contract.phase,
			});
		}
		if contract.role() != role {
			return Err(WorkerExecutionError::WrongRole {
				task,
				expected: role,
				actual: contract.role(),
			});
		}
		if self.tasks[index].state != TaskState::Idle {
			return Err(WorkerExecutionError::DuplicateDispatch(task));
		}
		self.ensure_dependencies(index)?;
		if let Some(active) = self.tasks.iter().find(|slot| {
			matches!(slot.state, TaskState::Active | TaskState::AwaitingAck)
				&& !slot.contract.window.overlaps(contract.window)
		}) {
			return Err(WorkerExecutionError::ScheduleConflict {
				task,
				active: active.contract.id,
			});
		}
		Ok(index)
	}

	fn ensure_dependencies(&self, index: usize) -> Result<(), WorkerExecutionError<B::Error>> {
		let contract = &self.tasks[index].contract;
		for dependency in &contract.projected_dependencies {
			let dependency_index = self.task_index(*dependency)?;
			if self.tasks[dependency_index].state != TaskState::Complete {
				return Err(WorkerExecutionError::DependencyIncomplete {
					task: contract.id,
					dependency: *dependency,
				});
			}
		}
		Ok(())
	}

	fn quiesce(&mut self) -> Result<(), WorkerExecutionError<B::Error>> {
		let mut calls = PhysicalCallBatch::new();
		let result = {
			let resource = self
				.resource
				.as_mut()
				.ok_or(WorkerExecutionError::InvalidLifecycle {
					state: self.lifecycle,
					detail: "worker resources are unavailable",
				})?;
			self.backend.quiesce_worker(resource, &mut calls)
		};
		self.backend_result(WorkerBackendOperation::Quiesce, calls, result)?;
		for slot in &mut self.tasks {
			if matches!(slot.state, TaskState::Active | TaskState::AwaitingAck) {
				slot.state = TaskState::Quiesced;
			}
		}
		Self::journal_result(
			self.journal
				.record_logical(LogicalEvent::WorkerQuiesced { run: self.run }),
		)
	}

	fn release_one_arena(&mut self, device: DeviceId) -> Result<(), WorkerExecutionError<B::Error>> {
		let image_index = self.image_index(device)?;
		if self.images[image_index].state == ImageState::Released {
			return Err(WorkerExecutionError::ArenaAlreadyReleased(device));
		}
		let arena = self
			.arenas
			.remove(&device)
			.ok_or(WorkerExecutionError::ArenaAlreadyReleased(device))?;
		let mut calls = PhysicalCallBatch::new();
		let result = {
			let resource = self
				.resource
				.as_mut()
				.ok_or(WorkerExecutionError::InvalidLifecycle {
					state: self.lifecycle,
					detail: "worker resources are unavailable",
				})?;
			self.backend
				.release_arena(resource, device, arena, &mut calls)
		};
		self.backend_result(WorkerBackendOperation::ReleaseArena(device), calls, result)?;
		self.images[image_index].state = ImageState::Released;
		self.images[image_index].buffer.fill(0);
		Self::journal_result(
			self.journal
				.record_logical(LogicalEvent::ArenaReleased { device }),
		)
	}

	fn bump_watchdog(&mut self, index: usize) -> Result<(), WorkerExecutionError<B::Error>> {
		let slot = &mut self.tasks[index];
		slot.nonprogress_polls = slot.nonprogress_polls.saturating_add(1);
		if slot.nonprogress_polls >= self.watchdog.max_nonprogress_polls() {
			return Err(WorkerExecutionError::WatchdogExpired {
				task: slot.contract.id,
				polls: slot.nonprogress_polls,
			});
		}
		Ok(())
	}

	fn ensure_run(&self, run: RunId) -> Result<(), WorkerExecutionError<B::Error>> {
		match run == self.run {
			true => Ok(()),
			false => {
				Err(WorkerExecutionError::RunMismatch {
					expected: self.run,
					actual: run,
				})
			}
		}
	}

	fn active_phase(&self) -> Result<RunPhase, WorkerExecutionError<B::Error>> {
		match self.lifecycle {
			WorkerLifecycle::Loop => Ok(RunPhase::Loop),
			WorkerLifecycle::Exit => Ok(RunPhase::Exit),
			state => {
				Err(WorkerExecutionError::InvalidLifecycle {
					state,
					detail: "worker has no dispatchable phase",
				})
			}
		}
	}

	fn lifecycle_error(&self, detail: &'static str) -> WorkerExecutionError<B::Error> {
		WorkerExecutionError::InvalidLifecycle {
			state: self.lifecycle,
			detail,
		}
	}

	fn task_index(&self, task: TaskId) -> Result<usize, WorkerExecutionError<B::Error>> {
		self.tasks
			.binary_search_by_key(&task, |slot| slot.contract.id)
			.map_err(|_| WorkerExecutionError::UnknownTask(task))
	}

	fn image_index(&self, device: DeviceId) -> Result<usize, WorkerExecutionError<B::Error>> {
		self.images
			.binary_search_by_key(&device, |image| image.device)
			.map_err(|_| WorkerExecutionError::UnknownDevice(device))
	}

	fn external_contract(&self, index: usize) -> Result<&WorkerExternalTransfer, WorkerExecutionError<B::Error>> {
		match &self.tasks[index].contract.work {
			PreparedWorkerWork::External(transfer) => Ok(transfer),
			_ => {
				Err(WorkerExecutionError::WrongRole {
					task: self.tasks[index].contract.id,
					expected: WorkerTaskRole::ExternalIngress,
					actual: self.tasks[index].contract.role(),
				})
			}
		}
	}

	fn backend_result<T>(
		&mut self,
		operation: WorkerBackendOperation,
		calls: PhysicalCallBatch,
		result: Result<T, B::Error>,
	) -> Result<T, WorkerExecutionError<B::Error>> {
		self.journal
			.record_physical(calls)
			.map_err(WorkerExecutionError::Journal)?;
		result.map_err(|source| WorkerExecutionError::Backend { operation, source })
	}

	fn journal_result<T>(result: crate::Result<T>) -> Result<T, WorkerExecutionError<B::Error>> {
		result.map_err(WorkerExecutionError::Journal)
	}
}

fn prepare_images<B: WorkerBackend>(
	projection: &WorkerProjection,
) -> Result<Box<[WorkerImage]>, WorkerExecutionError<B::Error>> {
	let mut images = Vec::with_capacity(projection.devices.len());
	for layout in &projection.layouts {
		let (task, image_bytes) = projection
			.tasks
			.iter()
			.find_map(|task| {
				match task.work {
					PreparedWorkerWork::InitAdmission { device, bytes, .. } if device == layout.device => {
						Some((task, bytes))
					}
					_ => None,
				}
			})
			.ok_or(WorkerExecutionError::Projection(
				WorkerProjectionError::MissingInitAdmission(layout.device),
			))?;
		let bytes = usize::try_from(image_bytes.get()).map_err(|_| WorkerExecutionError::CapacityOverflow)?;
		let mut buffer = Vec::new();
		buffer.try_reserve_exact(bytes)
			.map_err(|_| WorkerExecutionError::CapacityOverflow)?;
		buffer.resize(bytes, 0);
		images.push(WorkerImage {
			device: layout.device,
			task: task.id,
			bytes: image_bytes,
			expected_digest: Digest::ZERO,
			received: 0,
			buffer: buffer.into_boxed_slice(),
			state: ImageState::Needed,
		});
	}
	images.sort_by_key(|image| image.device);
	Ok(images.into_boxed_slice())
}

fn worker_journal_capacity<B: WorkerBackend>(
	bundle: &FinalizedBundle,
	projection: &WorkerProjection,
) -> Result<JournalCapacity, WorkerExecutionError<B::Error>> {
	let base = JournalCapacity::for_bundle::<B>(bundle).map_err(WorkerExecutionError::Journal)?;
	let external = projection.external_transfers().count();
	let extra_logical = external
		.checked_mul(2)
		.and_then(|count| count.checked_add(2))
		.ok_or(WorkerExecutionError::CapacityOverflow)?;
	let extra_physical = projection
		.tasks
		.len()
		.checked_mul(B::MAX_NON_POLL_PHYSICAL_CALLS)
		.and_then(|count| count.checked_add(B::MAX_NON_POLL_PHYSICAL_CALLS))
		.ok_or(WorkerExecutionError::CapacityOverflow)?;
	Ok(JournalCapacity::new(
		base.logical_events
			.checked_add(extra_logical)
			.ok_or(WorkerExecutionError::CapacityOverflow)?,
		base.physical_calls
			.checked_add(extra_physical)
			.ok_or(WorkerExecutionError::CapacityOverflow)?,
	))
}

fn preparation_failure<B: WorkerBackend>(
	mut backend: B,
	resource: B::Resource,
	mut journal: RunJournal,
	error: WorkerExecutionError<B::Error>,
) -> WorkerPrepareFailure<B> {
	let cleanup_error = destroy_prepared_resource(&mut backend, resource, &mut journal);
	WorkerPrepareFailure {
		error: Box::new(error),
		cleanup_error: cleanup_error.map(Box::new),
		backend,
	}
}

fn destroy_prepared_resource<B: WorkerBackend>(
	backend: &mut B,
	mut resource: B::Resource,
	journal: &mut RunJournal,
) -> Option<WorkerExecutionError<B::Error>> {
	let mut first_error = None;
	let mut calls = PhysicalCallBatch::new();
	let quiesce = backend.quiesce_worker(&mut resource, &mut calls);
	let record = journal.record_physical(calls);
	if let Err(source) = quiesce {
		first_error = Some(WorkerExecutionError::Backend {
			operation: WorkerBackendOperation::Quiesce,
			source,
		});
	}
	if let Err(error) = record
		&& first_error.is_none()
	{
		first_error = Some(WorkerExecutionError::Journal(error));
	}
	let mut calls = PhysicalCallBatch::new();
	let destroy = backend.destroy_resources(resource, &mut calls);
	let record = journal.record_physical(calls);
	if let Err(source) = destroy
		&& first_error.is_none()
	{
		first_error = Some(WorkerExecutionError::Backend {
			operation: WorkerBackendOperation::DestroyResources,
			source,
		});
	}
	if let Err(error) = record
		&& first_error.is_none()
	{
		first_error = Some(WorkerExecutionError::Journal(error));
	}
	first_error
}

fn byte_count<E: Error + Send + Sync + 'static>(bytes: usize) -> Result<ByteCount, WorkerExecutionError<E>> {
	u64::try_from(bytes)
		.map(ByteCount::new)
		.map_err(|_| WorkerExecutionError::CapacityOverflow)
}
