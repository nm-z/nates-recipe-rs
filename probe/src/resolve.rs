use std::collections::BTreeMap;

use recipe_core::{DeviceId, DeviceKind, MachineId};

use crate::{GpuDescriptor, GpuInventory, HostInventory, MeasuredProfile, ProbeError, ProbeResult, RamDomain, StorageDomain, validate_profile};

#[derive(Clone, Copy, Debug)]
pub struct ResolvedRamDomain<'inventory> {
	device: DeviceId,
	domain: &'inventory RamDomain,
}

impl<'inventory> ResolvedRamDomain<'inventory> {
	#[must_use]
	pub const fn device(self) -> DeviceId { self.device }

	#[must_use]
	pub const fn domain(self) -> &'inventory RamDomain { self.domain }
}

#[derive(Clone, Copy, Debug)]
pub struct ResolvedStorageDomain<'inventory> {
	device: DeviceId,
	domain: &'inventory StorageDomain,
}

impl<'inventory> ResolvedStorageDomain<'inventory> {
	#[must_use]
	pub const fn device(self) -> DeviceId { self.device }

	#[must_use]
	pub const fn domain(self) -> &'inventory StorageDomain { self.domain }
}

#[derive(Clone, Copy, Debug)]
pub struct ResolvedGpuDevice<'inventory> {
	device: DeviceId,
	descriptor: &'inventory GpuDescriptor,
}

impl<'inventory> ResolvedGpuDevice<'inventory> {
	#[must_use]
	pub const fn device(self) -> DeviceId { self.device }

	#[must_use]
	pub const fn descriptor(self) -> &'inventory GpuDescriptor { self.descriptor }
}

/// Exact association between one current bare-metal inventory and the
/// topology IDs retained in a validated measured profile.
#[derive(Clone, Debug)]
pub struct ResolvedLocalInventory<'inventory> {
	machine: MachineId,
	ram: Vec<ResolvedRamDomain<'inventory>>,
	storage: Vec<ResolvedStorageDomain<'inventory>>,
	gpu: Vec<ResolvedGpuDevice<'inventory>>,
}

impl<'inventory> ResolvedLocalInventory<'inventory> {
	#[must_use]
	pub const fn machine(&self) -> MachineId { self.machine }

	#[must_use]
	pub fn ram(&self) -> &[ResolvedRamDomain<'inventory>] { &self.ram }

	#[must_use]
	pub fn storage(&self) -> &[ResolvedStorageDomain<'inventory>] { &self.storage }

	#[must_use]
	pub fn gpu(&self) -> &[ResolvedGpuDevice<'inventory>] { &self.gpu }
}

impl MeasuredProfile {
	/// Reassociate the current exhaustive inventory with topology IDs using
	/// only probe-retained stable identities.
	///
	/// Capacity, product name, ordinal, and benchmark similarity are never
	/// fallback selectors. Missing or newly visible domains invalidate
	/// realization and require a fresh `recipe probe`.
	pub fn resolve_local_inventory<'inventory>(&self, host: &'inventory HostInventory, gpus: &'inventory GpuInventory) -> ProbeResult<ResolvedLocalInventory<'inventory>> {
		validate_profile(self)?;
		if !gpus.exhaustive {
			return Err(profile_mismatch("current GPU inventory is not exhaustive"));
		}
		let machine = self
			.origins
			.machines
			.iter()
			.find(|origin| origin.fingerprint == host.machine)
			.map(|origin| origin.machine)
			.ok_or_else(|| {
				profile_mismatch(format!(
					"current machine fingerprint {} has no exact profile origin",
					host.machine.stable_id
				))
			})?;

		let ram = resolve_ram(self, host, machine)?;
		let storage = resolve_storage(self, host, machine, &ram)?;
		let gpu = resolve_gpu(self, gpus, machine, &ram)?;
		Ok(ResolvedLocalInventory {
			machine,
			ram,
			storage,
			gpu,
		})
	}
}

fn resolve_ram<'inventory>(profile: &MeasuredProfile, host: &'inventory HostInventory, machine: MachineId) -> ProbeResult<Vec<ResolvedRamDomain<'inventory>>> {
	let expected = profile
		.origins
		.ram
		.iter()
		.filter_map(|origin| {
			profile
				.topology
				.device(origin.device)
				.filter(|device| device.machine == machine)
				.map(|_| (origin.key.as_str(), origin.device))
		})
		.collect::<BTreeMap<_, _>>();
	let live = host
		.ram
		.iter()
		.map(|domain| (domain.key.as_str(), domain))
		.collect::<BTreeMap<_, _>>();
	require_unique_live_keys("RAM", host.ram.len(), live.len())?;
	require_same_keys("RAM", &expected, &live)?;
	expected
		.into_iter()
		.map(|(key, device)| {
			let domain = live[key];
			let topology = profile
				.topology
				.device(device)
				.ok_or_else(|| profile_mismatch(format!("RAM device {device} disappeared from topology")))?;
			if topology.kind != DeviceKind::Ram {
				return Err(profile_mismatch(format!(
					"RAM origin {key} resolves to {:?} device {device}",
					topology.kind
				)));
			}
			Ok(ResolvedRamDomain { device, domain })
		})
		.collect()
}

fn resolve_storage<'inventory>(profile: &MeasuredProfile, host: &'inventory HostInventory, machine: MachineId, ram: &[ResolvedRamDomain<'inventory>]) -> ProbeResult<Vec<ResolvedStorageDomain<'inventory>>> {
	let expected = profile
		.origins
		.storage
		.iter()
		.filter_map(|origin| {
			profile
				.topology
				.device(origin.device)
				.filter(|device| device.machine == machine)
				.map(|_| (origin.key.as_str(), origin.device))
		})
		.collect::<BTreeMap<_, _>>();
	let live = host
		.storage
		.iter()
		.map(|domain| (domain.key.as_str(), domain))
		.collect::<BTreeMap<_, _>>();
	require_unique_live_keys("storage", host.storage.len(), live.len())?;
	require_same_keys("storage", &expected, &live)?;
	expected
		.into_iter()
		.map(|(key, device)| {
			let domain = live[key];
			require_host_memory("storage", key, &domain.host_memory_key, ram)?;
			Ok(ResolvedStorageDomain { device, domain })
		})
		.collect()
}

fn resolve_gpu<'inventory>(profile: &MeasuredProfile, gpus: &'inventory GpuInventory, machine: MachineId, ram: &[ResolvedRamDomain<'inventory>]) -> ProbeResult<Vec<ResolvedGpuDevice<'inventory>>> {
	let expected = profile
		.origins
		.gpu
		.iter()
		.filter_map(|origin| {
			profile
				.topology
				.device(origin.device)
				.filter(|device| device.machine == machine)
				.map(|_| (origin.key.as_str(), origin.device))
		})
		.collect::<BTreeMap<_, _>>();
	let live = gpus
		.devices
		.iter()
		.map(|descriptor| (descriptor.key.as_str(), descriptor))
		.collect::<BTreeMap<_, _>>();
	require_unique_live_keys("GPU", gpus.devices.len(), live.len())?;
	require_same_keys("GPU", &expected, &live)?;
	expected
		.into_iter()
		.map(|(key, device)| {
			let descriptor = live[key];
			require_host_memory("GPU", key, &descriptor.host_memory_key, ram)?;
			let discovered = profile.discovery.device(device).ok_or_else(|| {
				profile_mismatch(format!(
					"GPU origin {key} resolves to device {device} without discovery capability"
				))
			})?;
			let target = discovered
				.calculation
				.as_ref()
				.map(|calculation| &calculation.target)
				.ok_or_else(|| {
					profile_mismatch(format!(
						"GPU origin {key} resolves to device {device} without calculation capability"
					))
				})?;
			if target != &descriptor.target {
				return Err(profile_mismatch(format!(
					"GPU origin {key} target changed from {target:?} to {:?}",
					descriptor.target
				)));
			}
			Ok(ResolvedGpuDevice { device, descriptor })
		})
		.collect()
}

fn require_host_memory(kind: &str, key: &str, host_memory_key: &recipe_core::Label, ram: &[ResolvedRamDomain<'_>]) -> ProbeResult<()> {
	if ram.iter()
		.any(|resolved| resolved.domain.key == *host_memory_key)
	{
		Ok(())
	} else {
		Err(profile_mismatch(format!(
			"{kind} origin {key} references current RAM key {host_memory_key}, which is absent from the measured machine"
		)))
	}
}

fn require_same_keys<T, U>(kind: &str, expected: &BTreeMap<&str, T>, live: &BTreeMap<&str, U>) -> ProbeResult<()> {
	let missing = expected
		.keys()
		.filter(|key| !live.contains_key(**key))
		.copied()
		.collect::<Vec<_>>();
	let unexpected = live
		.keys()
		.filter(|key| !expected.contains_key(**key))
		.copied()
		.collect::<Vec<_>>();
	if missing.is_empty() && unexpected.is_empty() {
		Ok(())
	} else {
		Err(profile_mismatch(format!(
			"current {kind} origins differ from the measured profile: missing={missing:?}, unexpected={unexpected:?}"
		)))
	}
}

fn require_unique_live_keys(kind: &str, original_count: usize, unique_count: usize) -> ProbeResult<()> {
	if original_count == unique_count {
		Ok(())
	} else {
		Err(profile_mismatch(format!(
			"current {kind} inventory contains duplicate stable keys"
		)))
	}
}

fn profile_mismatch(detail: impl Into<String>) -> ProbeError { ProbeError::InvalidProfile(detail.into()) }
