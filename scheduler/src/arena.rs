use std::collections::BTreeMap;

use recipe_core::{ ArenaAllocation, ArenaLayout, ArenaObject, ByteCount, ByteOffset, CapacityLedger, DeviceId, Topology,
};

use crate::{ScheduleError, ScheduleErrorKind};

/// Pack all statically bounded objects into deterministic per-device arenas.
///
/// Objects whose half-open lifetimes do not overlap may reuse bytes. The
/// lowest aligned legal offset is chosen, with stable ID tie-breaking.
pub fn pack_arenas( topology: &Topology, objects: &[ArenaObject], capacity: &CapacityLedger,
) -> Result<Vec<ArenaLayout>, ScheduleError> { let mut by_device = BTreeMap::<DeviceId, Vec<&ArenaObject>>::new();
	for object in objects { if topology.device(object.device).is_none() { return Err(ScheduleError::new(
				ScheduleErrorKind::InvalidTopology,
				format!("arena object {} references an unknown device", object.id),
			) .for_device(object.device)); }
		if object.alignment.get() == 0 || !object.alignment.get().is_power_of_two() { return Err(ScheduleError::new(
				ScheduleErrorKind::InvalidTransfer,
				format!("arena object {} has invalid alignment", object.id),
			) .for_device(object.device)); }
		if !object.lifetime.is_valid() { return Err(ScheduleError::new( ScheduleErrorKind::InvalidTransfer,
				format!("arena object {} has an empty lifetime", object.id),
			) .for_device(object.device)); }
		by_device.entry(object.device).or_default().push(object); }

	let mut layouts = Vec::with_capacity(topology.devices.len()); let mut devices = topology .devices .iter()
		.map(|device| device.id) .collect::<Vec<_>>(); devices.sort(); for device in devices {
		let mut device_objects = by_device.remove(&device).unwrap_or_default();
		device_objects.sort_by_key(|object| (object.lifetime.start, object.id));
		let mut placed = Vec::<(&ArenaObject, ByteOffset)>::new();

		for object in device_objects { let mut candidates = vec![ByteCount::ZERO]; for (other, offset) in &placed {
				if object.lifetime.overlaps(other.lifetime) { let end = offset.checked_end(other.bytes).ok_or_else(|| {
						ScheduleError::new( ScheduleErrorKind::ArithmeticOverflow,
							"arena allocation end overflowed",
						) .for_device(device) })?; candidates.push(end); } }
			candidates.sort(); candidates.dedup();

			let mut selected = None; for candidate in candidates { let aligned = candidate .checked_align_up(object.alignment)
					.map_err(|error| { ScheduleError::new(ScheduleErrorKind::ArithmeticOverflow, error.to_string()) .for_device(device)
					})?; let offset = ByteOffset::new(aligned.get()); let end = offset.checked_end(object.bytes).ok_or_else(|| {
					ScheduleError::new( ScheduleErrorKind::ArithmeticOverflow,
						"arena allocation end overflowed",
					) .for_device(device) })?; let collision = placed.iter().any(|(other, other_offset)| {
					if !object.lifetime.overlaps(other.lifetime) { return false; }
					let other_end = other_offset .checked_end(other.bytes)
						.expect("previously checked allocation");
					ByteCount::new(offset.get()) < other_end && ByteCount::new(other_offset.get()) < end }); if !collision {
					selected = Some(offset); break; } }
			let offset = selected.ok_or_else(|| { ScheduleError::new( ScheduleErrorKind::ArithmeticOverflow,
					"no representable arena offset remains",
				) .for_device(device) })?; placed.push((object, offset)); }

		placed.sort_by_key(|(object, _)| object.id); let mut size = ByteCount::ZERO;
		let mut allocations = Vec::with_capacity(placed.len()); for (object, offset) in placed {
			size = size.max(offset.checked_end(object.bytes).ok_or_else(|| { ScheduleError::new(
					ScheduleErrorKind::ArithmeticOverflow,
					"arena allocation end overflowed",
				) .for_device(device) })?); allocations.push(ArenaAllocation { object: object.id, offset, }); }
		let usable = capacity.entry(device).ok_or_else(|| { ScheduleError::new( ScheduleErrorKind::InsufficientCapacity,
				"device has no realized capacity ledger entry",
			) .for_device(device) })?; if size > usable.recipe_usable.value { return Err(ScheduleError::new(
				ScheduleErrorKind::InsufficientCapacity, format!(
					"planned arena {} exceeds Recipe-usable capacity {}",
					size, usable.recipe_usable.value ), ) .for_device(device)); }
		layouts.push(ArenaLayout { device, size, allocations, }); }
	Ok(layouts) }
