use std::{
	cmp::Reverse,
	collections::{BTreeMap, BTreeSet, BinaryHeap},
};

use recipe_core::{
	CalculationTask, DeviceId, DiscoveryProfile, DuplexMode, DuplexResourceId, LinkId, Nanoseconds, RunPhase,
	ScheduleWindow, SubmissionSlots, Task, TaskId, TaskKind, Topology, TransferEndpoint, TransferLaneClaim,
	TransferTask, calculation_time_ceil, transfer_time_ceil,
};

use crate::{ScheduleError, ScheduleErrorKind, shortest_route};

/// A fully lowered task whose placement and logical resource slots are fixed,
/// but whose direct route (when omitted) and schedule window remain to be
/// selected. An indirect path must already be split into hop tasks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnscheduledTask {
	pub id: TaskId,
	pub phase: RunPhase,
	pub dependencies: Vec<TaskId>,
	pub kind: TaskKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StaticSchedule {
	pub tasks: Vec<Task>,
	pub makespan: Nanoseconds,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Resource {
	Queue(recipe_core::QueueSlotId),
	Completion(recipe_core::CompletionSlotId),
	ComputeLane(DeviceId, u32),
	TransferLane(LinkId, u32),
	ExternalTransferLane(DeviceId, u32),
	HalfDuplexDirection(DuplexResourceId, LinkId),
	NoComputeTransferOverlap(DeviceId),
}

#[derive(Clone, Debug)]
struct PreparedTask {
	task: UnscheduledTask,
	duration: Nanoseconds,
	resources: Vec<Resource>,
}

type Successors = BTreeMap<TaskId, Vec<TaskId>>;
type Indegrees = BTreeMap<TaskId, usize>;

/// Deterministic critical-path list scheduling over measured properties.
///
/// Every distinct-device internal transfer is exactly one directed-link hop.
/// An empty route may be filled automatically only when the selected route is
/// direct; callers must lower longer paths into dependency-chained hop tasks
/// with intermediate values before scheduling. Queue and completion slots are
/// exclusive, compute concurrency comes from the measured discovery profile,
/// opposing links contend exactly when they share a duplex resource, and
/// transfer/compute overlap follows discovered capability.
pub fn schedule(
	topology: &Topology,
	discovery: &DiscoveryProfile,
	tasks: &[UnscheduledTask],
) -> Result<StaticSchedule, ScheduleError> {
	topology
		.validate()
		.map_err(|errors| ScheduleError::new(ScheduleErrorKind::InvalidTopology, errors.to_string()))?;
	topology
		.validate_scheduling_properties()
		.map_err(|errors| ScheduleError::new(ScheduleErrorKind::InvalidTopology, errors.to_string()))?;
	discovery
		.validate(topology)
		.map_err(|errors| ScheduleError::new(ScheduleErrorKind::InvalidDiscovery, errors.to_string()))?;

	let prepared = prepare_tasks(topology, discovery, tasks)?;
	let (successors, indegrees) = dependency_graph(&prepared)?;
	let critical_paths = critical_path_lengths(&prepared, &successors, &indegrees)?;

	let mut remaining_dependencies = indegrees;
	let mut ready = BinaryHeap::<(u64, Reverse<TaskId>)>::new();
	for (id, count) in &remaining_dependencies {
		if *count == 0 {
			ready.push((critical_paths[id], Reverse(*id)));
		}
	}

	let mut task_end = BTreeMap::<TaskId, Nanoseconds>::new();
	let mut scheduled = BTreeMap::<TaskId, Task>::new();
	let mut reservations = BTreeMap::<Resource, Vec<ScheduleWindow>>::new();
	let mut phase_end = [Nanoseconds::ZERO; 3];

	while let Some((_, Reverse(id))) = ready.pop() {
		let prepared_task = prepared.get(&id).expect("ready tasks are prepared");
		let dependency_end = prepared_task
			.task
			.dependencies
			.iter()
			.filter_map(|dependency| task_end.get(dependency))
			.copied()
			.max()
			.unwrap_or(Nanoseconds::ZERO);
		let phase_floor = match prepared_task.task.phase {
			RunPhase::Init => Nanoseconds::ZERO,
			RunPhase::Loop => phase_end[0],
			RunPhase::Exit => phase_end[0].max(phase_end[1]),
		};
		let earliest = dependency_end.max(phase_floor);
		let (window, selected_resources) = reserve_earliest(
			earliest,
			prepared_task.duration,
			&prepared_task.resources,
			&reservations,
		)?;
		for resource in &selected_resources {
			reservations.entry(*resource).or_default().push(window);
		}
		let mut kind = prepared_task.task.kind.clone();
		persist_transfer_lane_claims(id, &mut kind, &selected_resources)?;
		let phase_index = match prepared_task.task.phase {
			RunPhase::Init => 0,
			RunPhase::Loop => 1,
			RunPhase::Exit => 2,
		};
		phase_end[phase_index] = phase_end[phase_index].max(window.end);
		task_end.insert(id, window.end);
		scheduled.insert(id, Task {
			id,
			phase: prepared_task.task.phase,
			window,
			dependencies: prepared_task.task.dependencies.clone(),
			kind,
		});
		for successor in successors.get(&id).into_iter().flatten() {
			let count = remaining_dependencies
				.get_mut(successor)
				.expect("successor has indegree");
			*count -= 1;
			if *count == 0 {
				ready.push((critical_paths[successor], Reverse(*successor)));
			}
		}
	}

	if scheduled.len() != prepared.len() {
		return Err(ScheduleError::new(
			ScheduleErrorKind::DependencyCycle,
			"task dependency graph contains a cycle",
		));
	}
	let makespan = scheduled
		.values()
		.map(|task| task.window.end)
		.max()
		.unwrap_or(Nanoseconds::ZERO);
	Ok(StaticSchedule {
		tasks: scheduled.into_values().collect(),
		makespan,
	})
}

fn prepare_tasks(
	topology: &Topology,
	discovery: &DiscoveryProfile,
	tasks: &[UnscheduledTask],
) -> Result<BTreeMap<TaskId, PreparedTask>, ScheduleError> {
	let mut prepared = BTreeMap::new();
	for source_task in tasks {
		if prepared.contains_key(&source_task.id) {
			return Err(ScheduleError::new(
				ScheduleErrorKind::DuplicateTask,
				format!("task {} appears more than once", source_task.id),
			)
			.for_task(source_task.id));
		}
		let mut task = source_task.clone();
		let (duration, mut resources) = match &mut task.kind {
			TaskKind::Calculation(calculation) => prepare_calculation(topology, discovery, task.id, calculation)?,
			TaskKind::Transfer(transfer) => prepare_transfer(topology, discovery, task.id, transfer)?,
			TaskKind::Metric(metric) => (Nanoseconds::new(1), submission_resources(metric.submission)),
		};
		resources.sort();
		resources.dedup();
		prepared.insert(task.id, PreparedTask {
			task,
			duration,
			resources,
		});
	}
	Ok(prepared)
}

fn prepare_calculation(
	topology: &Topology,
	discovery: &DiscoveryProfile,
	task: TaskId,
	calculation: &CalculationTask,
) -> Result<(Nanoseconds, Vec<Resource>), ScheduleError> {
	let Some(device) = topology.device(calculation.device) else {
		return Err(ScheduleError::new(
			ScheduleErrorKind::InvalidCalculationPlacement,
			"calculation references an unknown device",
		)
		.for_task(task)
		.for_device(calculation.device));
	};
	if !device.is_calculation_target() {
		return Err(ScheduleError::new(
			ScheduleErrorKind::InvalidCalculationPlacement,
			"calculation was placed on a non-GPU storage device",
		)
		.for_task(task)
		.for_device(calculation.device));
	}
	let capability = discovery
		.device(calculation.device)
		.and_then(|device| device.calculation.as_ref())
		.ok_or_else(|| {
			ScheduleError::new(
				ScheduleErrorKind::UnavailableCapability,
				"calculation capability is unavailable",
			)
			.for_task(task)
			.for_device(calculation.device)
		})?;
	let duration = calculation_time_ceil(calculation.work, capability.rate.value)
		.map_err(|error| {
			ScheduleError::new(ScheduleErrorKind::ArithmeticOverflow, error.to_string()).for_task(task)
		})?
		.get()
		.max(1);
	let mut resources = submission_resources(calculation.submission);
	for lane in 0..capability.maximum_concurrent_tasks {
		resources.push(Resource::ComputeLane(calculation.device, lane));
	}
	if discovery
		.device(calculation.device)
		.is_some_and(|device| !device.transfer.overlaps_calculation)
	{
		resources.push(Resource::NoComputeTransferOverlap(calculation.device));
	}
	Ok((Nanoseconds::new(duration), resources))
}

fn prepare_transfer(
	topology: &Topology,
	discovery: &DiscoveryProfile,
	task: TaskId,
	transfer: &mut TransferTask,
) -> Result<(Nanoseconds, Vec<Resource>), ScheduleError> {
	if !transfer.lane_claims.is_empty() {
		return Err(ScheduleError::new(
			ScheduleErrorKind::InvalidTransfer,
			"unscheduled transfer must not contain a preselected lane claim",
		)
		.for_task(task));
	}
	let mut resources = submission_resources(transfer.submission);
	let duration = match (transfer.source, transfer.destination) {
		(
			TransferEndpoint::Device { device: source, .. },
			TransferEndpoint::Device {
				device: destination,
				..
			},
		) => {
			if transfer.route.is_empty() && source != destination {
				let route = shortest_route(topology, source, destination, transfer.bytes)?;
				if route.links.len() != 1 {
					return Err(ScheduleError::new(
						ScheduleErrorKind::InvalidTransfer,
						"multi-hop internal transfer must be lowered into one dependency-chained task per directed link",
					)
					.for_task(task));
				}
				transfer.route = route.links;
			}
			if transfer.route.len() > 1 {
				return Err(ScheduleError::new(
					ScheduleErrorKind::InvalidTransfer,
					"executor-visible internal transfer cannot contain more than one directed link",
				)
				.for_task(task));
			}
			topology
				.validate_route(source, destination, &transfer.route)
				.map_err(|errors| {
					ScheduleError::new(ScheduleErrorKind::InvalidTransfer, errors.to_string()).for_task(task)
				})?;
			let nanoseconds = match transfer.route.first().copied() {
				None => 1,
				Some(link_id) => {
					let link = topology.link(link_id).ok_or_else(|| {
						ScheduleError::new(
							ScheduleErrorKind::InvalidTransfer,
							format!("validated route references absent link {link_id}"),
						)
						.for_task(task)
					})?;
					let hop = transfer_time_ceil(transfer.bytes, link.bandwidth.value)
						.map_err(|error| {
							ScheduleError::new(ScheduleErrorKind::ArithmeticOverflow, error.to_string())
								.for_task(task)
						})?
						.get()
						.max(1);
					for lane in 0..link.maximum_inflight_transfers.value.get() {
						resources.push(Resource::TransferLane(link.id, lane));
					}
					if link.duplex == DuplexMode::Half {
						resources.push(Resource::HalfDuplexDirection(
							link.capacity_resource,
							link.id,
						));
					}
					hop
				}
			};
			for endpoint in [source, destination] {
				let device = discovery.device(endpoint).ok_or_else(|| {
					ScheduleError::new(
						ScheduleErrorKind::UnavailableCapability,
						"transfer endpoint was not discovered",
					)
					.for_task(task)
					.for_device(endpoint)
				})?;
				if !device.transfer.overlaps_calculation {
					resources.push(Resource::NoComputeTransferOverlap(endpoint));
				}
			}
			Nanoseconds::new(nanoseconds)
		}
		(TransferEndpoint::External, TransferEndpoint::Device { device, .. })
		| (TransferEndpoint::Device { device, .. }, TransferEndpoint::External) => {
			if !transfer.route.is_empty() {
				return Err(ScheduleError::new(
					ScheduleErrorKind::InvalidTransfer,
					"external transfer cannot contain an internal route",
				)
				.for_task(task));
			}
			let capability = discovery.device(device).ok_or_else(|| {
				ScheduleError::new(
					ScheduleErrorKind::UnavailableCapability,
					"external transfer endpoint was not discovered",
				)
				.for_task(task)
				.for_device(device)
			})?;
			if !capability.transfer.overlaps_calculation {
				resources.push(Resource::NoComputeTransferOverlap(device));
			}
			for lane in 0..capability.transfer.maximum_inflight_transfers.value.get() {
				resources.push(Resource::ExternalTransferLane(device, lane));
			}
			Nanoseconds::new(
				transfer_time_ceil(transfer.bytes, capability.transfer.rate.value)
					.map_err(|error| {
						ScheduleError::new(ScheduleErrorKind::ArithmeticOverflow, error.to_string())
							.for_task(task)
					})?
					.get()
					.max(1),
			)
		}
		(TransferEndpoint::External, TransferEndpoint::External) => {
			return Err(ScheduleError::new(
				ScheduleErrorKind::InvalidTransfer,
				"external-to-external transfer is not schedulable",
			)
			.for_task(task));
		}
	};
	Ok((duration, resources))
}

fn persist_transfer_lane_claims(
	task: TaskId,
	kind: &mut TaskKind,
	selected_resources: &[Resource],
) -> Result<(), ScheduleError> {
	let TaskKind::Transfer(transfer) = kind else {
		return Ok(());
	};
	let mut claims = selected_resources
		.iter()
		.filter_map(|resource| {
			match resource {
				Resource::TransferLane(link, lane) => {
					Some(TransferLaneClaim::Link {
						link: *link,
						lane: *lane,
					})
				}
				Resource::ExternalTransferLane(device, lane) => {
					Some(TransferLaneClaim::External {
						device: *device,
						lane: *lane,
					})
				}
				Resource::Queue(_)
				| Resource::Completion(_)
				| Resource::ComputeLane(_, _)
				| Resource::HalfDuplexDirection(_, _)
				| Resource::NoComputeTransferOverlap(_) => None,
			}
		})
		.collect::<Vec<_>>();
	claims.sort();
	claims.dedup();
	let valid = match (transfer.source, transfer.destination) {
		(TransferEndpoint::Device { .. }, TransferEndpoint::Device { .. }) => {
			let expected = transfer.route.iter().copied().collect::<BTreeSet<_>>();
			let claimed = claims
				.iter()
				.filter_map(|claim| {
					match claim {
						TransferLaneClaim::Link { link, .. } => Some(*link),
						TransferLaneClaim::External { .. } => None,
					}
				})
				.collect::<BTreeSet<_>>();
			claims.iter()
				.all(|claim| matches!(claim, TransferLaneClaim::Link { .. }))
				&& claimed == expected
		}
		(TransferEndpoint::External, TransferEndpoint::Device { device, .. })
		| (TransferEndpoint::Device { device, .. }, TransferEndpoint::External) => {
			matches!(
				claims.as_slice(),
				[TransferLaneClaim::External {
					device: claimed,
					..
				}] if *claimed == device
			)
		}
		(TransferEndpoint::External, TransferEndpoint::External) => false,
	};
	if !valid {
		return Err(ScheduleError::new(
			ScheduleErrorKind::UnavailableCapability,
			"scheduler did not select the complete transfer lane claim set",
		)
		.for_task(task));
	}
	transfer.lane_claims = claims;
	Ok(())
}

fn submission_resources(submission: SubmissionSlots) -> Vec<Resource> {
	vec![
		Resource::Queue(submission.queue),
		Resource::Completion(submission.completion),
	]
}

fn dependency_graph(tasks: &BTreeMap<TaskId, PreparedTask>) -> Result<(Successors, Indegrees), ScheduleError> {
	let mut edges = BTreeSet::<(TaskId, TaskId)>::new();
	for prepared in tasks.values() {
		let mut unique = BTreeSet::new();
		for dependency in &prepared.task.dependencies {
			if !tasks.contains_key(dependency) {
				return Err(ScheduleError::new(
					ScheduleErrorKind::UnknownDependency,
					format!("unknown dependency {dependency}"),
				)
				.for_task(prepared.task.id));
			}
			if !unique.insert(*dependency) {
				return Err(ScheduleError::new(
					ScheduleErrorKind::UnknownDependency,
					format!("dependency {dependency} is repeated"),
				)
				.for_task(prepared.task.id));
			}
			let predecessor = &tasks[dependency].task;
			if predecessor.phase > prepared.task.phase {
				return Err(ScheduleError::new(
					ScheduleErrorKind::InvalidLifecycleDependency,
					"task depends on a later lifecycle phase",
				)
				.for_task(prepared.task.id));
			}
			edges.insert((*dependency, prepared.task.id));
		}
	}

	// Lifecycle phases are global barriers. Making these edges explicit only in
	// the scheduling graph prevents a ready Loop task from starting while a
	// lower-priority Init task is still unscheduled.
	for predecessor in tasks.values() {
		for successor in tasks.values() {
			if predecessor.task.phase < successor.task.phase {
				edges.insert((predecessor.task.id, successor.task.id));
			}
		}
	}

	let mut successors = Successors::new();
	let mut indegrees = tasks.keys().map(|id| (*id, 0usize)).collect::<Indegrees>();
	for (predecessor, successor) in edges {
		successors.entry(predecessor).or_default().push(successor);
		*indegrees.get_mut(&successor).expect("task key exists") += 1;
	}
	for list in successors.values_mut() {
		list.sort();
	}
	Ok((successors, indegrees))
}

fn critical_path_lengths(
	tasks: &BTreeMap<TaskId, PreparedTask>,
	successors: &BTreeMap<TaskId, Vec<TaskId>>,
	indegrees: &BTreeMap<TaskId, usize>,
) -> Result<BTreeMap<TaskId, u64>, ScheduleError> {
	let mut counts = indegrees.clone();
	let mut ready = BinaryHeap::<Reverse<TaskId>>::new();
	for (id, count) in &counts {
		if *count == 0 {
			ready.push(Reverse(*id));
		}
	}
	let mut order = Vec::with_capacity(tasks.len());
	while let Some(Reverse(id)) = ready.pop() {
		order.push(id);
		for successor in successors.get(&id).into_iter().flatten() {
			let count = counts.get_mut(successor).expect("successor key exists");
			*count -= 1;
			if *count == 0 {
				ready.push(Reverse(*successor));
			}
		}
	}
	if order.len() != tasks.len() {
		return Err(ScheduleError::new(
			ScheduleErrorKind::DependencyCycle,
			"task dependency graph contains a cycle",
		));
	}
	let mut lengths = BTreeMap::<TaskId, u64>::new();
	for id in order.into_iter().rev() {
		let successor = successors
			.get(&id)
			.into_iter()
			.flatten()
			.map(|successor| lengths[successor])
			.max()
			.unwrap_or(0);
		let length = tasks[&id]
			.duration
			.get()
			.checked_add(successor)
			.ok_or_else(|| {
				ScheduleError::new(
					ScheduleErrorKind::ArithmeticOverflow,
					"critical-path duration overflowed",
				)
			})?;
		lengths.insert(id, length);
	}
	Ok(lengths)
}

fn reserve_earliest(
	earliest: Nanoseconds,
	duration: Nanoseconds,
	resources: &[Resource],
	reservations: &BTreeMap<Resource, Vec<ScheduleWindow>>,
) -> Result<(ScheduleWindow, Vec<Resource>), ScheduleError> {
	let fixed = resources
		.iter()
		.filter(|resource| resource_group(resource).is_none())
		.copied()
		.collect::<Vec<_>>();
	let groups = resources
		.iter()
		.filter_map(resource_group)
		.collect::<BTreeSet<_>>();
	first_gap(earliest, duration, &fixed, &groups, resources, reservations)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum ResourceGroup {
	Compute(DeviceId),
	Transfer(LinkId),
	ExternalTransfer(DeviceId),
}

const fn resource_group(resource: &Resource) -> Option<ResourceGroup> {
	match resource {
		Resource::ComputeLane(device, _) => Some(ResourceGroup::Compute(*device)),
		Resource::TransferLane(link, _) => Some(ResourceGroup::Transfer(*link)),
		Resource::ExternalTransferLane(device, _) => Some(ResourceGroup::ExternalTransfer(*device)),
		Resource::Queue(_)
		| Resource::Completion(_)
		| Resource::HalfDuplexDirection(_, _)
		| Resource::NoComputeTransferOverlap(_) => None,
	}
}

fn first_gap(
	earliest: Nanoseconds,
	duration: Nanoseconds,
	fixed: &[Resource],
	groups: &BTreeSet<ResourceGroup>,
	resources: &[Resource],
	reservations: &BTreeMap<Resource, Vec<ScheduleWindow>>,
) -> Result<(ScheduleWindow, Vec<Resource>), ScheduleError> {
	let mut start = earliest;
	loop {
		let end = start.checked_add(duration).ok_or_else(|| {
			ScheduleError::new(
				ScheduleErrorKind::ArithmeticOverflow,
				"schedule window overflowed",
			)
		})?;
		let candidate = ScheduleWindow { start, end };
		let fixed_conflict = fixed
			.iter()
			.filter_map(|resource| resource_conflict_end(*resource, candidate, reservations))
			.max();
		if let Some(next) = fixed_conflict {
			start = next;
			continue;
		}
		let mut selected = fixed.to_vec();
		let mut unavailable_until = None;
		for group in groups {
			let lanes = resources
				.iter()
				.filter(|resource| resource_group(resource) == Some(*group));
			let mut first_available = None;
			let mut earliest_lane_release = None;
			for lane in lanes {
				match resource_conflict_end(*lane, candidate, reservations) {
					Some(end) => {
						earliest_lane_release =
							Some(earliest_lane_release.map_or(end, |known: Nanoseconds| known.min(end)));
					}
					None => {
						first_available = Some(*lane);
						break;
					}
				}
			}
			match first_available {
				Some(lane) => selected.push(lane),
				None => {
					let release = earliest_lane_release.ok_or_else(|| {
						ScheduleError::new(
							ScheduleErrorKind::UnavailableCapability,
							"task has an empty transfer or compute lane group",
						)
					})?;
					unavailable_until =
						Some(unavailable_until.map_or(release, |known: Nanoseconds| known.min(release)));
				}
			}
		}
		match unavailable_until {
			Some(next) => start = next,
			None => {
				selected.sort();
				return Ok((candidate, selected));
			}
		}
	}
}

fn resource_conflict_end(
	resource: Resource,
	candidate: ScheduleWindow,
	reservations: &BTreeMap<Resource, Vec<ScheduleWindow>>,
) -> Option<Nanoseconds> {
	match resource {
		Resource::HalfDuplexDirection(capacity, direction) => {
			reservations
				.iter()
				.filter_map(|(reserved, windows)| {
					match reserved {
						Resource::HalfDuplexDirection(reserved_capacity, reserved_direction)
							if *reserved_capacity == capacity && *reserved_direction != direction =>
						{
							Some(windows)
						}
						_ => None,
					}
				})
				.flatten()
				.filter(|window| window.overlaps(candidate))
				.map(|window| window.end)
				.max()
		}
		_ => {
			reservations
				.get(&resource)
				.into_iter()
				.flatten()
				.filter(|window| window.overlaps(candidate))
				.map(|window| window.end)
				.max()
		}
	}
}
