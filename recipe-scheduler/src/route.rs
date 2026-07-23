use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap};

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

#[cfg(test)]
mod tests {
	use recipe_core::{
		ByteCount, BytesPerSecond, Device, DeviceKind, DirectedLink, DuplexMode, DuplexResourceId, Machine,
		MachineId, Node, NodeId, NodeRole, Property, PropertyProvenance, Topology, TopologyIdentity,
		TransferLaneCount, TransportId, TransportKind,
	};
	use recipe_core::{Digest, Label};

	use super::*;

	fn measured<T>(value: T) -> Property<T> {
		Property::new(value, PropertyProvenance::Measured)
	}

	fn fixture() -> Topology {
		let machine = MachineId::new(1);
		let devices = (1..=3).map(DeviceId::new).collect::<Vec<_>>();
		let link = |id, transport, from, to, bandwidth, resource| DirectedLink {
			id: LinkId::new(id),
			transport: TransportId::new(transport),
			kind: TransportKind::Memory,
			duplex: DuplexMode::Full,
			from,
			to,
			bandwidth: measured(BytesPerSecond::new(bandwidth).unwrap()),
			maximum_inflight_transfers: measured(TransferLaneCount::new(1).unwrap()),
			capacity_resource: DuplexResourceId::new(resource),
		};
		Topology {
			identity: TopologyIdentity::new(Digest::new([1; 32])),
			machines: vec![Machine {
				id: machine,
				name: Label::new("host").unwrap(),
			}],
			nodes: vec![Node {
				id: NodeId::new(1),
				role: NodeRole::Master,
				machine,
				devices: devices.clone(),
			}],
			devices: devices
				.iter()
				.copied()
				.map(|id| Device {
					id,
					machine,
					kind: DeviceKind::Ram,
					capacity: measured(ByteCount::new(8_000_000_000)),
					transfer_rate: measured(BytesPerSecond::new(1_000_000_000).unwrap()),
					calculation_rate: None,
				})
				.collect(),
			links: vec![
				link(1, 1, devices[0], devices[2], 100, 1),
				link(2, 2, devices[0], devices[1], 1_000, 3),
				link(3, 3, devices[1], devices[2], 1_000, 5),
				link(4, 1, devices[2], devices[0], 100, 2),
				link(5, 2, devices[1], devices[0], 1_000, 4),
				link(6, 3, devices[2], devices[1], 1_000, 6),
			],
		}
	}

	#[test]
	fn chooses_minimum_measured_time_not_fewest_hops() {
		let route = shortest_route(
			&fixture(),
			DeviceId::new(1),
			DeviceId::new(3),
			ByteCount::new(1_000),
		)
		.unwrap();
		assert_eq!(route.links, vec![LinkId::new(2), LinkId::new(3)]);
		assert_eq!(route.duration, Nanoseconds::new(2_000_000_000));
	}

	#[test]
	fn rejects_estimates_as_production_inputs() {
		let mut topology = fixture();
		topology.links[0].bandwidth.provenance = PropertyProvenance::Estimated;
		let error = shortest_route(
			&topology,
			DeviceId::new(1),
			DeviceId::new(3),
			ByteCount::new(1),
		)
		.unwrap_err();
		assert_eq!(error.kind, ScheduleErrorKind::InvalidTopology);

		topology.links[0].bandwidth.provenance = PropertyProvenance::Measured;
		topology.links[0].maximum_inflight_transfers.provenance = PropertyProvenance::Estimated;
		let error = shortest_route(
			&topology,
			DeviceId::new(1),
			DeviceId::new(3),
			ByteCount::new(1),
		)
		.unwrap_err();
		assert_eq!(error.kind, ScheduleErrorKind::InvalidTopology);
	}
}
