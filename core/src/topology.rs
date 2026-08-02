use std::collections::{BTreeMap, BTreeSet};

use crate::{
	error::{ValidationCode, ValidationResult, Validator},
	identity::{Label, TopologyIdentity},
	ids::{DeviceId, DuplexResourceId, LinkId, MachineId, NodeId, TransportId},
	units::{ByteCount, BytesPerSecond, FlopsPerSecond, TransferLaneCount},
};

/// Origin of a declared or discovered property.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PropertyProvenance {
	Estimated,
	Measured,
	Override,
}

/// A value whose origin is never implicit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Property<T> {
	pub value: T,
	pub provenance: PropertyProvenance,
}

impl<T> Property<T> {
	#[must_use]
	pub const fn new(value: T, provenance: PropertyProvenance) -> Self { Self { value, provenance } }

	/// Whether this value is permitted to drive a production schedule.
	#[must_use]
	pub const fn is_schedulable(&self) -> bool {
		matches!(
			self.provenance,
			PropertyProvenance::Measured | PropertyProvenance::Override
		)
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DeviceKind {
	GpuMemory,
	Ram,
	Disk,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Machine {
	pub id: MachineId,
	pub name: Label,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NodeRole {
	Master,
	Worker,
}

/// Logical run ownership of a set of storage devices.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
	pub id: NodeId,
	pub role: NodeRole,
	pub machine: MachineId,
	pub devices: Vec<DeviceId>,
}

/// One schedulable storage domain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Device {
	pub id: DeviceId,
	pub machine: MachineId,
	pub kind: DeviceKind,
	pub capacity: Property<ByteCount>,
	pub transfer_rate: Property<BytesPerSecond>,
	/// Present only for a GPU calculation target.
	pub calculation_rate: Option<Property<FlopsPerSecond>>,
}

impl Device {
	#[must_use]
	pub fn is_calculation_target(&self) -> bool { self.kind == DeviceKind::GpuMemory }
}

/// Physical transport family used by one bidirectional link pair.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TransportKind {
	Memory,
	Pcie,
	Sata,
	Sas,
	Nvme,
	Ethernet,
	Wlan,
}

impl TransportKind {
	#[must_use]
	pub const fn required_duplex(self) -> DuplexMode {
		match self {
			Self::Sata | Self::Wlan => DuplexMode::Half,
			Self::Memory | Self::Pcie | Self::Sas | Self::Nvme | Self::Ethernet => DuplexMode::Full,
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DuplexMode {
	Half,
	Full,
}

/// One measured direction of a bidirectional physical transport.
///
/// Both directions share a `transport` identity. Half-duplex directions share
/// one `capacity_resource`; full-duplex directions have distinct resources.
/// `maximum_inflight_transfers` is directional and therefore may differ from
/// the reverse edge when that difference was measured. When the value exceeds
/// one, `bandwidth` is the measured effective per-transfer rate while that many
/// same-direction transfers overlap; discovery must conservatively report one
/// until a concurrent measurement establishes a larger value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirectedLink {
	pub id: LinkId,
	pub transport: TransportId,
	pub kind: TransportKind,
	pub duplex: DuplexMode,
	pub from: DeviceId,
	pub to: DeviceId,
	pub bandwidth: Property<BytesPerSecond>,
	pub maximum_inflight_transfers: Property<TransferLaneCount>,
	pub capacity_resource: DuplexResourceId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Topology {
	pub identity: TopologyIdentity,
	pub machines: Vec<Machine>,
	pub nodes: Vec<Node>,
	pub devices: Vec<Device>,
	pub links: Vec<DirectedLink>,
}

impl Topology {
	pub fn validate(&self) -> ValidationResult {
		let mut validator = Validator::default();
		validator.require(
			!self.identity.is_zero(),
			ValidationCode::InvalidIdentity,
			"identity",
			"topology identity must not be zero",
		);

		let mut machine_ids = BTreeSet::new();
		let mut machine_names = BTreeSet::new();
		for (index, machine) in self.machines.iter().enumerate() {
			validator.require(
				machine_ids.insert(machine.id),
				ValidationCode::DuplicateId,
				format!("machines[{index}].id"),
				format!("machine id {} appears more than once", machine.id),
			);
			validator.require(
				machine_names.insert(machine.name.clone()),
				ValidationCode::DuplicateName,
				format!("machines[{index}].name"),
				format!("machine name {} appears more than once", machine.name),
			);
		}

		let mut device_ids = BTreeSet::new();
		let mut devices_per_machine = BTreeMap::<MachineId, usize>::new();
		for (index, device) in self.devices.iter().enumerate() {
			validator.require(
				device_ids.insert(device.id),
				ValidationCode::DuplicateId,
				format!("devices[{index}].id"),
				format!("device id {} appears more than once", device.id),
			);
			validator.require(
				machine_ids.contains(&device.machine),
				ValidationCode::UnknownReference,
				format!("devices[{index}].machine"),
				format!("unknown machine {}", device.machine),
			);
			*devices_per_machine.entry(device.machine).or_default() += 1;
			match device.kind {
				DeviceKind::GpuMemory => {
					validator.require(
						device.calculation_rate.is_some(),
						ValidationCode::MissingRequiredObject,
						format!("devices[{index}].calculation_rate"),
						"GPU storage device requires a calculation rate",
					)
				}
				DeviceKind::Ram | DeviceKind::Disk => {
					validator.require(
						device.calculation_rate.is_none(),
						ValidationCode::WrongKind,
						format!("devices[{index}].calculation_rate"),
						"non-GPU storage device cannot be a calculation target",
					)
				}
			}
		}
		for (index, machine) in self.machines.iter().enumerate() {
			validator.require(
				devices_per_machine.get(&machine.id).copied().unwrap_or(0) != 0,
				ValidationCode::MissingRequiredObject,
				format!("machines[{index}].devices"),
				"configured machine must declare at least one storage device",
			);
		}

		let mut node_ids = BTreeSet::new();
		let mut master_count = 0usize;
		let mut owners = BTreeMap::<DeviceId, NodeId>::new();
		for (node_index, node) in self.nodes.iter().enumerate() {
			validator.require(
				node_ids.insert(node.id),
				ValidationCode::DuplicateId,
				format!("nodes[{node_index}].id"),
				format!("node id {} appears more than once", node.id),
			);
			if node.role == NodeRole::Master {
				master_count += 1;
			}
			validator.require(
				machine_ids.contains(&node.machine),
				ValidationCode::UnknownReference,
				format!("nodes[{node_index}].machine"),
				format!("unknown machine {}", node.machine),
			);
			let mut local = BTreeSet::new();
			for (device_index, device_id) in node.devices.iter().copied().enumerate() {
				validator.require(
					local.insert(device_id),
					ValidationCode::DuplicateId,
					format!("nodes[{node_index}].devices[{device_index}]"),
					format!("device {device_id} is repeated in the node"),
				);
				let device = self.devices.iter().find(|device| device.id == device_id);
				validator.require(
					device.is_some(),
					ValidationCode::UnknownReference,
					format!("nodes[{node_index}].devices[{device_index}]"),
					format!("unknown device {device_id}"),
				);
				if let Some(device) = device {
					validator.require(
						device.machine == node.machine,
						ValidationCode::MachineMismatch,
						format!("nodes[{node_index}].devices[{device_index}]"),
						format!(
							"device {device_id} belongs to machine {}, not {}",
							device.machine, node.machine
						),
					);
				}
				if let Some(previous) = owners.insert(device_id, node.id) {
					validator.error(
						ValidationCode::DeviceOwnedMultipleTimes,
						format!("nodes[{node_index}].devices[{device_index}]"),
						format!("device {device_id} is already owned by node {previous}"),
					);
				}
			}
		}
		match master_count {
			0 => {
				validator.error(
					ValidationCode::MissingMaster,
					"nodes",
					"topology requires exactly one master node",
				)
			}
			1 => {}
			_ => {
				validator.error(
					ValidationCode::MultipleMasters,
					"nodes",
					format!("topology has {master_count} master nodes"),
				)
			}
		}
		for (index, device) in self.devices.iter().enumerate() {
			validator.require(
				owners.contains_key(&device.id),
				ValidationCode::UnownedDevice,
				format!("devices[{index}]"),
				format!("required device {} is not bound to a node", device.id),
			);
		}

		let mut link_ids = BTreeSet::new();
		let mut transports = BTreeMap::<TransportId, Vec<(usize, &DirectedLink)>>::new();
		for (index, link) in self.links.iter().enumerate() {
			validator.require(
				link_ids.insert(link.id),
				ValidationCode::DuplicateId,
				format!("links[{index}].id"),
				format!("link id {} appears more than once", link.id),
			);
			validator.require(
				device_ids.contains(&link.from),
				ValidationCode::UnknownReference,
				format!("links[{index}].from"),
				format!("unknown source device {}", link.from),
			);
			validator.require(
				device_ids.contains(&link.to),
				ValidationCode::UnknownReference,
				format!("links[{index}].to"),
				format!("unknown destination device {}", link.to),
			);
			validator.require(
				link.from != link.to,
				ValidationCode::InvalidRoute,
				format!("links[{index}]"),
				"a directed link must connect distinct devices",
			);
			validator.require(
				link.transport.get() != 0,
				ValidationCode::InvalidTransport,
				format!("links[{index}].transport"),
				"transport identity must be nonzero",
			);
			validator.require(
				link.capacity_resource.get() != 0,
				ValidationCode::InvalidDuplex,
				format!("links[{index}].capacity_resource"),
				"duplex capacity resource must be nonzero",
			);
			validator.require(
				link.duplex == link.kind.required_duplex(),
				ValidationCode::InvalidDuplex,
				format!("links[{index}].duplex"),
				format!(
					"{:?} transport requires {:?} duplex operation",
					link.kind,
					link.kind.required_duplex()
				),
			);
			transports
				.entry(link.transport)
				.or_default()
				.push((index, link));
		}

		let mut resource_owners = BTreeMap::<DuplexResourceId, TransportId>::new();
		for (transport, directions) in &transports {
			validator.require(
				directions.len() == 2,
				ValidationCode::InvalidTransport,
				"links",
				format!(
					"transport {transport} must have exactly two directed edges, found {}",
					directions.len()
				),
			);
			if let [(first_index, first), (second_index, second)] = directions.as_slice() {
				validator.require(
					first.from == second.to && first.to == second.from,
					ValidationCode::InvalidTransport,
					format!("links[{second_index}]"),
					format!(
						"transport {transport} edge {} at links[{second_index}] is not the reverse of edge {} at links[{first_index}]",
						second.id, first.id,
					),
				);
				validator.require(
					first.kind == second.kind,
					ValidationCode::InvalidTransport,
					format!("links[{second_index}].kind"),
					format!("transport {transport} declares asymmetric transport kinds"),
				);
				validator.require(
					first.duplex == second.duplex,
					ValidationCode::InvalidDuplex,
					format!("links[{second_index}].duplex"),
					format!("transport {transport} declares asymmetric duplex modes"),
				);
				match first.duplex {
					DuplexMode::Half => {
						validator.require(
							first.capacity_resource == second.capacity_resource,
							ValidationCode::InvalidDuplex,
							format!("links[{second_index}].capacity_resource"),
							format!("half-duplex transport {transport} must share one capacity resource"),
						)
					}
					DuplexMode::Full => {
						validator.require(
							first.capacity_resource != second.capacity_resource,
							ValidationCode::InvalidDuplex,
							format!("links[{second_index}].capacity_resource"),
							format!(
								"full-duplex transport {transport} requires one capacity resource per direction"
							),
						)
					}
				}
			}
			for (index, direction) in directions {
				if let Some(owner) = resource_owners.insert(direction.capacity_resource, *transport) {
					validator.require(
						owner == *transport,
						ValidationCode::InvalidDuplex,
						format!("links[{index}].capacity_resource"),
						format!(
							"capacity resource {} is shared by transports {owner} and {transport}",
							direction.capacity_resource
						),
					);
				}
			}
		}

		validator.finish()
	}

	/// Rejects theoretical seed estimates before Draft scheduling.
	///
	/// Estimated values may size probing workloads, but only measured values or
	/// explicit overrides may participate in a production schedule.
	pub fn validate_scheduling_properties(&self) -> ValidationResult {
		let mut validator = Validator::default();
		for (index, device) in self.devices.iter().enumerate() {
			validator.require(
				device.capacity.is_schedulable(),
				ValidationCode::UnmeasuredProperty,
				format!("devices[{index}].capacity"),
				"production device capacity must be measured or explicitly overridden",
			);
			validator.require(
				device.transfer_rate.is_schedulable(),
				ValidationCode::UnmeasuredProperty,
				format!("devices[{index}].transfer_rate"),
				"production device transfer rate must be measured or explicitly overridden",
			);
			if let Some(rate) = &device.calculation_rate {
				validator.require(
					rate.is_schedulable(),
					ValidationCode::UnmeasuredProperty,
					format!("devices[{index}].calculation_rate"),
					"production calculation rate must be measured or explicitly overridden",
				);
			}
		}
		for (index, link) in self.links.iter().enumerate() {
			validator.require(
				link.bandwidth.is_schedulable(),
				ValidationCode::UnmeasuredProperty,
				format!("links[{index}].bandwidth"),
				"production link bandwidth must be measured or explicitly overridden",
			);
			validator.require(
				link.maximum_inflight_transfers.is_schedulable(),
				ValidationCode::UnmeasuredProperty,
				format!("links[{index}].maximum_inflight_transfers"),
				"production link concurrency must be measured or explicitly overridden",
			);
		}
		validator.finish()
	}

	#[must_use]
	pub fn machine(&self, id: MachineId) -> Option<&Machine> { self.machines.iter().find(|machine| machine.id == id) }

	#[must_use]
	pub fn device(&self, id: DeviceId) -> Option<&Device> { self.devices.iter().find(|device| device.id == id) }

	#[must_use]
	pub fn link(&self, id: LinkId) -> Option<&DirectedLink> { self.links.iter().find(|link| link.id == id) }

	#[must_use]
	pub fn duplex_mode(&self, first: LinkId, second: LinkId) -> Option<DuplexMode> {
		let first = self.link(first)?;
		let second = self.link(second)?;
		if first.transport != second.transport || first.from != second.to || first.to != second.from {
			return None;
		}
		if first.duplex != second.duplex {
			return None;
		}
		match first.duplex {
			DuplexMode::Half if first.capacity_resource == second.capacity_resource => Some(DuplexMode::Half),
			DuplexMode::Full if first.capacity_resource != second.capacity_resource => Some(DuplexMode::Full),
			DuplexMode::Half | DuplexMode::Full => None,
		}
	}

	pub fn validate_route(&self, source: DeviceId, destination: DeviceId, route: &[LinkId]) -> ValidationResult {
		if source == destination && route.is_empty() {
			return Ok(());
		}
		let mut errors = Validator::default();
		let mut current = source;
		for (index, link_id) in route.iter().copied().enumerate() {
			let Some(link) = self.link(link_id) else {
				errors.error(
					ValidationCode::UnknownReference,
					format!("route[{index}]"),
					format!("unknown link {link_id}"),
				);
				continue;
			};
			if link.from != current {
				errors.error(
					ValidationCode::RouteEndpointMismatch,
					format!("route[{index}]"),
					format!("link {link_id} starts at {}, expected {current}", link.from),
				);
			}
			current = link.to;
		}
		errors.require(
			!route.is_empty(),
			ValidationCode::InvalidRoute,
			"route",
			"a transfer between distinct devices requires at least one link",
		);
		errors.require(
			current == destination,
			ValidationCode::RouteEndpointMismatch,
			"route",
			format!("route ends at {current}, expected {destination}"),
		);
		errors.finish()
	}
}
