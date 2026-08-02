use std::{
	cmp::Reverse,
	collections::{BTreeMap, BinaryHeap},
};

use recipe_core::{ByteCount, DeviceId, LinkId, Nanoseconds, Topology, transfer_time_ceil};

use crate::{ScheduleError, ScheduleErrorKind};

/// A deterministic planner path candidate and its store-and-forward time.
///
/// A route with multiple links is not one executor task. The planner lowers it
/// to one dependency-chained transfer task per link before static scheduling.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Route {
	pub links: Vec<LinkId>,
	pub duration: Nanoseconds,
}

/// Find the minimum measured transfer-time route.
///
/// Equal-duration paths are ordered lexicographically by their stable link IDs.
/// Each hop is charged `ceil(bytes / measured_bandwidth)` and the candidate path
/// is costed conservatively as store-and-forward.
pub fn shortest_route(
	topology: &Topology,
	source: DeviceId,
	destination: DeviceId,
	bytes: ByteCount,
) -> Result<Route, ScheduleError> {
	topology
		.validate()
		.map_err(|errors| ScheduleError::new(ScheduleErrorKind::InvalidTopology, errors.to_string()))?;
	topology
		.validate_scheduling_properties()
		.map_err(|errors| ScheduleError::new(ScheduleErrorKind::InvalidTopology, errors.to_string()))?;

	if topology.device(source).is_none() {
		return Err(ScheduleError::new(
			ScheduleErrorKind::InvalidTransfer,
			format!("unknown source device {source}"),
		)
		.for_device(source));
	}
	if topology.device(destination).is_none() {
		return Err(ScheduleError::new(
			ScheduleErrorKind::InvalidTransfer,
			format!("unknown destination device {destination}"),
		)
		.for_device(destination));
	}
	if source == destination {
		return Ok(Route {
			links: Vec::new(),
			duration: Nanoseconds::new(1),
		});
	}

	let mut adjacency = BTreeMap::<DeviceId, Vec<_>>::new();
	for link in &topology.links {
		adjacency.entry(link.from).or_default().push(link);
	}
	for links in adjacency.values_mut() {
		links.sort_by_key(|link| link.id);
	}

	let mut best = BTreeMap::<DeviceId, (u64, Vec<LinkId>)>::new();
	let mut ready = BinaryHeap::<Reverse<(u64, Vec<LinkId>, DeviceId)>>::new();
	best.insert(source, (0, Vec::new()));
	ready.push(Reverse((0, Vec::new(), source)));

	while let Some(Reverse((elapsed, path, current))) = ready.pop() {
		if best.get(&current) != Some(&(elapsed, path.clone())) {
			continue;
		}
		if current == destination {
			return Ok(Route {
				links: path,
				duration: Nanoseconds::new(elapsed.max(1)),
			});
		}
		for link in adjacency.get(&current).into_iter().flatten() {
			let hop = transfer_time_ceil(bytes, link.bandwidth.value)
				.map_err(|error| ScheduleError::new(ScheduleErrorKind::ArithmeticOverflow, error.to_string()))?
				.get()
				.max(1);
			let candidate_elapsed = elapsed.checked_add(hop).ok_or_else(|| {
				ScheduleError::new(
					ScheduleErrorKind::ArithmeticOverflow,
					"route duration overflowed",
				)
			})?;
			let mut candidate_path = path.clone();
			candidate_path.push(link.id);
			let candidate = (candidate_elapsed, candidate_path);
			let replace = best
				.get(&link.to)
				.is_none_or(|known| candidate < known.clone());
			if replace {
				best.insert(link.to, candidate.clone());
				ready.push(Reverse((candidate.0, candidate.1, link.to)));
			}
		}
	}

	Err(ScheduleError::new(
		ScheduleErrorKind::NoRoute,
		format!("no directed route from {source} to {destination}"),
	))
}
