use std::collections::BTreeSet;

use crate::{
	artifact::TargetIdentity,
	error::{ValidationCode, ValidationResult, Validator},
	identity::{DiscoveryIdentity, TopologyIdentity},
	ids::{DeviceId, LinkId},
	topology::{DeviceKind, Property, Topology},
	units::{ByteCount, BytesPerSecond, FlopsPerSecond, TransferLaneCount},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CalculationCapability {
	pub target: TargetIdentity,
	pub rate: Property<FlopsPerSecond>,
	pub asynchronous_submission: bool,
	pub maximum_concurrent_tasks: u32,
	/// Native SIMD/SIMT subgroup width reported by the calculation backend.
	pub subgroup_lanes: u32,
	/// Maximum lanes permitted in one native workgroup.
	pub maximum_workgroup_lanes: u32,
	/// Maximum workgroup-local/shared storage reported by the backend.
	pub maximum_shared_memory_per_workgroup: ByteCount,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TransferCapability {
	pub rate: Property<BytesPerSecond>,
	pub maximum_inflight_transfers: Property<TransferLaneCount>,
	pub asynchronous_submission: bool,
	pub overlaps_calculation: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiscoveredDevice {
	pub device: DeviceId,
	pub available: bool,
	pub maximum_submission_queues: u32,
	pub total_capacity: Property<ByteCount>,
	pub transfer: TransferCapability,
	pub calculation: Option<CalculationCapability>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiscoveredLink {
	pub link: LinkId,
	pub available: bool,
	pub bandwidth: Property<BytesPerSecond>,
	pub maximum_inflight_transfers: Property<TransferLaneCount>,
	pub asynchronous_submission: bool,
}

/// Immutable capability snapshot for all required topology objects.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiscoveryProfile {
	pub identity: DiscoveryIdentity,
	pub topology: TopologyIdentity,
	pub devices: Vec<DiscoveredDevice>,
	pub links: Vec<DiscoveredLink>,
}

impl DiscoveryProfile {
	pub fn validate(&self, topology: &Topology) -> ValidationResult {
		let mut validator = Validator::default();
		validator.require(
			!self.identity.is_zero(),
			ValidationCode::InvalidIdentity,
			"identity",
			"discovery identity must not be zero",
		);
		validator.require(
			self.topology == topology.identity,
			ValidationCode::IdentityMismatch,
			"topology",
			"discovery profile belongs to a different topology",
		);

		let required_devices = topology
			.devices
			.iter()
			.map(|device| device.id)
			.collect::<BTreeSet<_>>();
		let mut discovered_devices = BTreeSet::new();
		for (index, discovered) in self.devices.iter().enumerate() {
			validator.require(
				required_devices.contains(&discovered.device),
				ValidationCode::UnknownReference,
				format!("devices[{index}].device"),
				format!("unknown device {}", discovered.device),
			);
			validator.require(
				discovered_devices.insert(discovered.device),
				ValidationCode::DuplicateId,
				format!("devices[{index}].device"),
				format!("device {} appears more than once", discovered.device),
			);
			validator.require(
				discovered.available,
				ValidationCode::UnavailableRequiredObject,
				format!("devices[{index}].available"),
				format!("required device {} is unavailable", discovered.device),
			);
			validator.require(
				discovered.maximum_submission_queues != 0,
				ValidationCode::UnsupportedCapability,
				format!("devices[{index}].maximum_submission_queues"),
				"required device must support at least one submission queue",
			);
			validator.require(
				discovered.total_capacity.is_schedulable(),
				ValidationCode::UnmeasuredProperty,
				format!("devices[{index}].total_capacity"),
				"production discovery capacity must be measured or explicitly overridden",
			);
			validator.require(
				discovered.transfer.rate.is_schedulable(),
				ValidationCode::UnmeasuredProperty,
				format!("devices[{index}].transfer.rate"),
				"production transfer rate must be measured or explicitly overridden",
			);
			validator.require(
				discovered
					.transfer
					.maximum_inflight_transfers
					.is_schedulable(),
				ValidationCode::UnmeasuredProperty,
				format!("devices[{index}].transfer.maximum_inflight_transfers"),
				"production transfer concurrency must be measured or explicitly overridden",
			);
			validator.require(
				discovered.transfer.asynchronous_submission,
				ValidationCode::UnsupportedCapability,
				format!("devices[{index}].transfer"),
				"required device does not support asynchronous transfer submission",
			);
			if let Some(device) = topology.device(discovered.device) {
				match device.kind {
					DeviceKind::GpuMemory => {
						validator.require(
							discovered.calculation.is_some(),
							ValidationCode::MissingRequiredObject,
							format!("devices[{index}].calculation"),
							"GPU device requires calculation capability",
						);
						if let Some(calculation) = &discovered.calculation {
							validator.require(
								calculation.rate.is_schedulable(),
								ValidationCode::UnmeasuredProperty,
								format!("devices[{index}].calculation.rate"),
								"production calculation rate must be measured or explicitly overridden",
							);
							validator.require(
								calculation.asynchronous_submission,
								ValidationCode::UnsupportedCapability,
								format!("devices[{index}].calculation"),
								"GPU calculation submission must be asynchronous",
							);
							validator.require(
								calculation.maximum_concurrent_tasks != 0,
								ValidationCode::UnsupportedCapability,
								format!("devices[{index}].calculation.maximum_concurrent_tasks"),
								"GPU device must support at least one active calculation",
							);
							validator.require(
								calculation.subgroup_lanes != 0
									&& calculation.subgroup_lanes.is_power_of_two(),
								ValidationCode::UnsupportedCapability,
								format!("devices[{index}].calculation.subgroup_lanes"),
								"GPU subgroup width must be a nonzero power of two",
							);
							validator.require(
								calculation.maximum_workgroup_lanes >= calculation.subgroup_lanes,
								ValidationCode::UnsupportedCapability,
								format!("devices[{index}].calculation.maximum_workgroup_lanes"),
								"GPU workgroup limit must contain at least one subgroup",
							);
							validator.require(
								calculation.maximum_shared_memory_per_workgroup.get() != 0,
								ValidationCode::UnsupportedCapability,
								format!(
									"devices[{index}].calculation.maximum_shared_memory_per_workgroup"
								),
								"GPU workgroup-local memory limit must be nonzero",
							);
						}
					}
					DeviceKind::Ram | DeviceKind::Disk => {
						validator.require(
							discovered.calculation.is_none(),
							ValidationCode::WrongKind,
							format!("devices[{index}].calculation"),
							"non-GPU storage cannot expose calculation capability",
						)
					}
				}
			}
		}
		for (index, device) in topology.devices.iter().enumerate() {
			validator.require(
				discovered_devices.contains(&device.id),
				ValidationCode::MissingRequiredObject,
				format!("topology.devices[{index}]"),
				format!("required device {} was not discovered", device.id),
			);
		}

		let required_links = topology
			.links
			.iter()
			.map(|link| link.id)
			.collect::<BTreeSet<_>>();
		let mut discovered_links = BTreeSet::new();
		for (index, discovered) in self.links.iter().enumerate() {
			validator.require(
				required_links.contains(&discovered.link),
				ValidationCode::UnknownReference,
				format!("links[{index}].link"),
				format!("unknown link {}", discovered.link),
			);
			validator.require(
				discovered_links.insert(discovered.link),
				ValidationCode::DuplicateId,
				format!("links[{index}].link"),
				format!("link {} appears more than once", discovered.link),
			);
			validator.require(
				discovered.available,
				ValidationCode::UnavailableRequiredObject,
				format!("links[{index}].available"),
				format!("required link {} is unavailable", discovered.link),
			);
			validator.require(
				discovered.bandwidth.is_schedulable(),
				ValidationCode::UnmeasuredProperty,
				format!("links[{index}].bandwidth"),
				"production link bandwidth must be measured or explicitly overridden",
			);
			validator.require(
				discovered.maximum_inflight_transfers.is_schedulable(),
				ValidationCode::UnmeasuredProperty,
				format!("links[{index}].maximum_inflight_transfers"),
				"production link concurrency must be measured or explicitly overridden",
			);
			validator.require(
				discovered.asynchronous_submission,
				ValidationCode::UnsupportedCapability,
				format!("links[{index}].asynchronous_submission"),
				"required link does not support asynchronous transfer submission",
			);
			if let Some(link) = topology.link(discovered.link) {
				validator.require(
					discovered.bandwidth == link.bandwidth,
					ValidationCode::ResourceMismatch,
					format!("links[{index}].bandwidth"),
					"discovery bandwidth differs from the topology direction",
				);
				validator.require(
					discovered.maximum_inflight_transfers == link.maximum_inflight_transfers,
					ValidationCode::ResourceMismatch,
					format!("links[{index}].maximum_inflight_transfers"),
					"discovery concurrency differs from the topology direction",
				);
			}
		}
		for (index, link) in topology.links.iter().enumerate() {
			validator.require(
				discovered_links.contains(&link.id),
				ValidationCode::MissingRequiredObject,
				format!("topology.links[{index}]"),
				format!("required link {} was not discovered", link.id),
			);
		}

		validator.finish()
	}

	#[must_use]
	pub fn device(&self, id: DeviceId) -> Option<&DiscoveredDevice> {
		self.devices
			.iter()
			.find(|discovered| discovered.device == id)
	}
}
