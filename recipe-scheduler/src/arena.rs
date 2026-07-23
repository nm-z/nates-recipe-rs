use std::collections::BTreeMap;

use recipe_core::{
	ArenaAllocation, ArenaLayout, ArenaObject, ByteCount, ByteOffset, CapacityLedger, DeviceId, Topology,
};

use crate::{ScheduleError, ScheduleErrorKind};

/// Pack all statically bounded objects into deterministic per-device arenas.
///
/// Objects whose half-open lifetimes do not overlap may reuse bytes. The
/// lowest aligned legal offset is chosen, with stable ID tie-breaking.
pub fn pack_arenas(
	topology: &Topology,
	objects: &[ArenaObject],
	capacity: &CapacityLedger,
) -> Result<Vec<ArenaLayout>, ScheduleError> {
	let mut by_device = BTreeMap::<DeviceId, Vec<&ArenaObject>>::new();
	for object in objects {
		if topology.device(object.device).is_none() {
			return Err(ScheduleError::new(
				ScheduleErrorKind::InvalidTopology,
				format!("arena object {} references an unknown device", object.id),
			)
			.for_device(object.device));
		}
		if object.alignment.get() == 0 || !object.alignment.get().is_power_of_two() {
			return Err(ScheduleError::new(
				ScheduleErrorKind::InvalidTransfer,
				format!("arena object {} has invalid alignment", object.id),
			)
			.for_device(object.device));
		}
		if !object.lifetime.is_valid() {
			return Err(ScheduleError::new(
				ScheduleErrorKind::InvalidTransfer,
				format!("arena object {} has an empty lifetime", object.id),
			)
			.for_device(object.device));
		}
		by_device.entry(object.device).or_default().push(object);
	}

	let mut layouts = Vec::with_capacity(topology.devices.len());
	let mut devices = topology
		.devices
		.iter()
		.map(|device| device.id)
		.collect::<Vec<_>>();
	devices.sort();
	for device in devices {
		let mut device_objects = by_device.remove(&device).unwrap_or_default();
		device_objects.sort_by_key(|object| (object.lifetime.start, object.id));
		let mut placed = Vec::<(&ArenaObject, ByteOffset)>::new();

		for object in device_objects {
			let mut candidates = vec![ByteCount::ZERO];
			for (other, offset) in &placed {
				if object.lifetime.overlaps(other.lifetime) {
					let end = offset.checked_end(other.bytes).ok_or_else(|| {
						ScheduleError::new(
							ScheduleErrorKind::ArithmeticOverflow,
							"arena allocation end overflowed",
						)
						.for_device(device)
					})?;
					candidates.push(end);
				}
			}
			candidates.sort();
			candidates.dedup();

			let mut selected = None;
			for candidate in candidates {
				let aligned = candidate
					.checked_align_up(object.alignment)
					.map_err(|error| {
						ScheduleError::new(ScheduleErrorKind::ArithmeticOverflow, error.to_string())
							.for_device(device)
					})?;
				let offset = ByteOffset::new(aligned.get());
				let end = offset.checked_end(object.bytes).ok_or_else(|| {
					ScheduleError::new(
						ScheduleErrorKind::ArithmeticOverflow,
						"arena allocation end overflowed",
					)
					.for_device(device)
				})?;
				let collision = placed.iter().any(|(other, other_offset)| {
					if !object.lifetime.overlaps(other.lifetime) {
						return false;
					}
					let other_end = other_offset
						.checked_end(other.bytes)
						.expect("previously checked allocation");
					ByteCount::new(offset.get()) < other_end && ByteCount::new(other_offset.get()) < end
				});
				if !collision {
					selected = Some(offset);
					break;
				}
			}
			let offset = selected.ok_or_else(|| {
				ScheduleError::new(
					ScheduleErrorKind::ArithmeticOverflow,
					"no representable arena offset remains",
				)
				.for_device(device)
			})?;
			placed.push((object, offset));
		}

		placed.sort_by_key(|(object, _)| object.id);
		let mut size = ByteCount::ZERO;
		let mut allocations = Vec::with_capacity(placed.len());
		for (object, offset) in placed {
			size = size.max(offset.checked_end(object.bytes).ok_or_else(|| {
				ScheduleError::new(
					ScheduleErrorKind::ArithmeticOverflow,
					"arena allocation end overflowed",
				)
				.for_device(device)
			})?);
			allocations.push(ArenaAllocation {
				object: object.id,
				offset,
			});
		}
		let usable = capacity.entry(device).ok_or_else(|| {
			ScheduleError::new(
				ScheduleErrorKind::InsufficientCapacity,
				"device has no realized capacity ledger entry",
			)
			.for_device(device)
		})?;
		if size > usable.recipe_usable.value {
			return Err(ScheduleError::new(
				ScheduleErrorKind::InsufficientCapacity,
				format!(
					"planned arena {} exceeds Recipe-usable capacity {}",
					size, usable.recipe_usable.value
				),
			)
			.for_device(device));
		}
		layouts.push(ArenaLayout {
			device,
			size,
			allocations,
		});
	}
	Ok(layouts)
}

#[cfg(test)]
mod tests {
	use recipe_core::{
		ArenaObjectId, CapacityLedgerEntry, Device, DeviceKind, Digest, Label, Machine, MachineId, Nanoseconds,
		Node, NodeId, NodeRole, Property, PropertyProvenance, ScheduleWindow, TopologyIdentity,
	};

	use super::*;

	fn measured<T>(value: T) -> Property<T> {
		Property::new(value, PropertyProvenance::Measured)
	}

	fn fixture() -> (Topology, CapacityLedger) {
		let machine = MachineId::new(1);
		let device = DeviceId::new(1);
		let topology = Topology {
			identity: TopologyIdentity::new(Digest::new([1; 32])),
			machines: vec![Machine {
				id: machine,
				name: Label::new("host").unwrap(),
			}],
			nodes: vec![Node {
				id: NodeId::new(1),
				role: NodeRole::Master,
				machine,
				devices: vec![device],
			}],
			devices: vec![Device {
				id: device,
				machine,
				kind: DeviceKind::Ram,
				capacity: measured(ByteCount::new(10_000)),
				transfer_rate: measured(recipe_core::BytesPerSecond::new(1).unwrap()),
				calculation_rate: None,
			}],
			links: Vec::new(),
		};
		let capacity = CapacityLedger {
			entries: vec![CapacityLedgerEntry {
				device,
				total: measured(ByteCount::new(10_000)),
				runtime_overhead: measured(ByteCount::ZERO),
				fragmentation: measured(ByteCount::ZERO),
				safety_headroom: measured(ByteCount::ZERO),
				recipe_usable: measured(ByteCount::new(1_000)),
			}],
		};
		(topology, capacity)
	}

	fn object(id: u64, start: u64, end: u64, bytes: u64) -> ArenaObject {
		ArenaObject {
			id: ArenaObjectId::new(id),
			device: DeviceId::new(1),
			bytes: ByteCount::new(bytes),
			alignment: ByteCount::new(64),
			lifetime: ScheduleWindow {
				start: Nanoseconds::new(start),
				end: Nanoseconds::new(end),
			},
		}
	}

	#[test]
	fn reuses_memory_for_nonoverlapping_lifetimes() {
		let (topology, capacity) = fixture();
		let layouts = pack_arenas(
			&topology,
			&[object(1, 1, 5, 128), object(2, 5, 9, 128)],
			&capacity,
		)
		.unwrap();
		assert_eq!(layouts[0].size, ByteCount::new(128));
		assert_eq!(layouts[0].allocations[0].offset, ByteOffset::new(0));
		assert_eq!(layouts[0].allocations[1].offset, ByteOffset::new(0));
	}

	#[test]
	fn separates_simultaneously_live_objects_and_checks_capacity() {
		let (topology, mut capacity) = fixture();
		let objects = [object(1, 1, 9, 128), object(2, 2, 8, 128)];
		let layouts = pack_arenas(&topology, &objects, &capacity).unwrap();
		assert_eq!(layouts[0].size, ByteCount::new(256));

		capacity.entries[0].recipe_usable.value = ByteCount::new(255);
		let error = pack_arenas(&topology, &objects, &capacity).unwrap_err();
		assert_eq!(error.kind, ScheduleErrorKind::InsufficientCapacity);
	}
}
