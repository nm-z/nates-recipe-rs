use crate::abi::*;
use crate::identity::c_bool;
use crate::{
	AgentUuid, DeviceType, Error, IsaTarget, MemoryLocation, MemoryPoolFlags, MemorySegment, PciAddress, Profile,
	QueueKind, Result, RoundingModes, Runtime,
};
use core::ffi::c_void;
use core::mem::MaybeUninit;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::str::FromStr;

const MAX_ISA_NAME_BYTES: usize = 4096;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SystemDescription {
	pub hsa_version_major: u16,
	pub hsa_version_minor: u16,
	pub amd_extension_version_major: u16,
	pub amd_extension_version_minor: u16,
	pub timestamp_frequency_hz: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentIdentity {
	pub name: String,
	pub vendor_name: String,
	pub uuid: AgentUuid,
	pub numa_node_id: u32,
	pub driver_node_id: Option<u32>,
	pub pci_address: Option<PciAddress>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct QueueCapabilities {
	pub minimum_packets: u32,
	pub maximum_packets: u32,
	pub advertised_kind: QueueKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AmdGpuProperties {
	pub chip_id: u32,
	pub asic_revision: u32,
	pub cacheline_bytes: u32,
	pub compute_unit_count: u32,
	pub simds_per_compute_unit: u32,
	pub maximum_waves_per_compute_unit: u32,
	pub maximum_clock_mhz: u32,
	pub maximum_memory_clock_mhz: u32,
	pub product_name: String,
	pub available_memory_bytes: u64,
	pub timestamp_frequency_hz: u64,
	pub sdma_engine_count: u32,
	pub xgmi_sdma_engine_count: u32,
	pub xcc_count: u32,
	pub maximum_scratch_bytes: u64,
	pub current_scratch_limit_bytes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IsaDescription {
	/// Exact ROCr identity, unmodified.
	pub name: String,
	/// Parsed target identity for AMD GPU agents. Non-GPU ISAs have no AMD
	/// target and are never eligible for Recipe calculation placement.
	pub amd_target: Option<IsaTarget>,
	pub supports_small_machine_model: bool,
	pub supports_large_machine_model: bool,
	pub supports_base_profile: bool,
	pub supports_full_profile: bool,
	pub default_rounding_modes: RoundingModes,
	pub base_profile_rounding_modes: RoundingModes,
	pub fast_f16: bool,
	pub maximum_workgroup_dimensions: [u16; 3],
	pub maximum_workgroup_size: u32,
	pub maximum_grid_dimensions: [u32; 3],
	pub maximum_grid_size: u64,
	pub maximum_fbarriers_per_workgroup: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AllocationProperties {
	pub granule_bytes: usize,
	pub recommended_granule_bytes: usize,
	pub alignment_bytes: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryPoolDescription {
	pub segment: MemorySegment,
	/// Newer ROCr implementations expose this directly. `None` records that
	/// the runtime rejected the extension attribute; callers must not guess.
	pub location: Option<MemoryLocation>,
	pub global_flags: Option<MemoryPoolFlags>,
	pub size_bytes: usize,
	pub maximum_aggregate_allocation_bytes: Option<usize>,
	pub accessible_by_all_agents: bool,
	pub runtime_allocation: Option<AllocationProperties>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentDescription {
	pub identity: AgentIdentity,
	pub device_type: DeviceType,
	pub profile: Profile,
	/// Raw HSA feature bits, including any bits introduced by newer runtimes.
	pub feature_bits: u32,
	pub hsa_version_major: u16,
	pub hsa_version_minor: u16,
	/// Deprecated HSA agent query corresponding to the first enumerated ISA.
	pub first_isa_wavefront_size: Option<u32>,
	pub queue: Option<QueueCapabilities>,
	pub amd_gpu: Option<AmdGpuProperties>,
	pub isas: Vec<IsaDescription>,
	pub memory_pools: Vec<MemoryPoolDescription>,
}

impl AgentDescription {
	pub const fn supports_kernel_dispatch(&self) -> bool {
		self.feature_bits & AGENT_FEATURE_KERNEL_DISPATCH != 0
	}

	pub const fn supports_agent_dispatch(&self) -> bool {
		self.feature_bits & AGENT_FEATURE_AGENT_DISPATCH != 0
	}
}

#[derive(Clone, Copy)]
pub(crate) struct RawPool {
	pub(crate) handle: HsaMemoryPool,
}

pub struct DiscoveredAgent<'runtime> {
	pub(crate) runtime: &'runtime Runtime,
	pub(crate) raw_agent: HsaAgent,
	pub(crate) raw_pools: Vec<RawPool>,
	pub(crate) description: AgentDescription,
}

impl DiscoveredAgent<'_> {
	pub fn description(&self) -> &AgentDescription {
		&self.description
	}
}

pub struct Discovery<'runtime> {
	system: SystemDescription,
	agents: Vec<DiscoveredAgent<'runtime>>,
}

impl<'runtime> Discovery<'runtime> {
	pub fn system(&self) -> &SystemDescription {
		&self.system
	}

	pub fn agents(&self) -> &[DiscoveredAgent<'runtime>] {
		&self.agents
	}

	pub fn into_agents(self) -> Vec<DiscoveredAgent<'runtime>> {
		self.agents
	}
}

pub(crate) fn discover(runtime: &Runtime) -> Result<Discovery<'_>> {
	let api = &runtime.api;
	let system = SystemDescription {
		hsa_version_major: system_info(api, SYSTEM_INFO_VERSION_MAJOR, "HSA system major version")?,
		hsa_version_minor: system_info(api, SYSTEM_INFO_VERSION_MINOR, "HSA system minor version")?,
		amd_extension_version_major: system_info(
			api,
			AMD_SYSTEM_INFO_EXT_VERSION_MAJOR,
			"AMD HSA extension major version",
		)?,
		amd_extension_version_minor: system_info(
			api,
			AMD_SYSTEM_INFO_EXT_VERSION_MINOR,
			"AMD HSA extension minor version",
		)?,
		timestamp_frequency_hz: system_info(
			api,
			SYSTEM_INFO_TIMESTAMP_FREQUENCY,
			"HSA timestamp frequency",
		)?,
	};

	let raw_agents = iterate_agents(api)?;
	let mut agents = Vec::new();
	agents.try_reserve_exact(raw_agents.len())
		.map_err(|_| Error::AllocationFailed {
			field: "agent descriptions",
			requested: raw_agents.len(),
		})?;
	for raw_agent in raw_agents {
		agents.push(discover_agent(runtime, raw_agent)?);
	}

	Ok(Discovery { system, agents })
}

fn discover_agent(runtime: &Runtime, raw_agent: HsaAgent) -> Result<DiscoveredAgent<'_>> {
	let api = &runtime.api;
	let device_type = DeviceType::from_raw(agent_info(
		api,
		raw_agent,
		AGENT_INFO_DEVICE,
		"HSA agent device type",
	)?)?;
	let feature_bits = agent_info(api, raw_agent, AGENT_INFO_FEATURE, "HSA agent features")?;
	let profile = Profile::from_raw(
		agent_info(api, raw_agent, AGENT_INFO_PROFILE, "HSA agent profile")?,
		"HSA_AGENT_INFO_PROFILE",
	)?;
	let uuid_text = agent_string::<21>(api, raw_agent, AMD_AGENT_INFO_UUID, "AMD agent UUID")?;
	let uuid = AgentUuid::from_str(&uuid_text)?;
	if uuid.device_type() != device_type {
		return Err(Error::InvalidIdentity {
			kind: "agent UUID",
			value: uuid_text,
			reason: "device prefix disagrees with HSA_AGENT_INFO_DEVICE",
		});
	}

	let minimum_packets = agent_info(
		api,
		raw_agent,
		AGENT_INFO_QUEUE_MIN_SIZE,
		"HSA minimum queue size",
	)?;
	let maximum_packets = agent_info(
		api,
		raw_agent,
		AGENT_INFO_QUEUE_MAX_SIZE,
		"HSA maximum queue size",
	)?;
	let queue = if minimum_packets == 0 && maximum_packets == 0 {
		None
	} else {
		let capabilities = QueueCapabilities {
			minimum_packets,
			maximum_packets,
			advertised_kind: QueueKind::from_raw(agent_info(
				api,
				raw_agent,
				AGENT_INFO_QUEUE_TYPE,
				"HSA queue type",
			)?)?,
		};
		validate_queue_capabilities(capabilities)?;
		Some(capabilities)
	};

	let (driver_node_id, pci_address, amd_gpu) = if device_type == DeviceType::Gpu {
		let driver_node_id = agent_info(
			api,
			raw_agent,
			AMD_AGENT_INFO_DRIVER_NODE_ID,
			"AMD driver node ID",
		)?;
		let domain = agent_info(api, raw_agent, AMD_AGENT_INFO_DOMAIN, "AMD PCI domain")?;
		let bdf_id = agent_info(api, raw_agent, AMD_AGENT_INFO_BDFID, "AMD PCI BDF ID")?;
		let gpu = AmdGpuProperties {
			chip_id: agent_info(api, raw_agent, AMD_AGENT_INFO_CHIP_ID, "AMD chip ID")?,
			asic_revision: agent_info(
				api,
				raw_agent,
				AMD_AGENT_INFO_ASIC_REVISION,
				"AMD ASIC revision",
			)?,
			cacheline_bytes: agent_info(
				api,
				raw_agent,
				AMD_AGENT_INFO_CACHELINE_SIZE,
				"AMD cacheline size",
			)?,
			compute_unit_count: agent_info(
				api,
				raw_agent,
				AMD_AGENT_INFO_COMPUTE_UNIT_COUNT,
				"AMD compute-unit count",
			)?,
			simds_per_compute_unit: agent_info(
				api,
				raw_agent,
				AMD_AGENT_INFO_NUM_SIMDS_PER_CU,
				"AMD SIMDs per compute unit",
			)?,
			maximum_waves_per_compute_unit: agent_info(
				api,
				raw_agent,
				AMD_AGENT_INFO_MAX_WAVES_PER_CU,
				"AMD maximum waves per compute unit",
			)?,
			maximum_clock_mhz: agent_info(
				api,
				raw_agent,
				AMD_AGENT_INFO_MAX_CLOCK_FREQUENCY,
				"AMD maximum GPU clock",
			)?,
			maximum_memory_clock_mhz: agent_info(
				api,
				raw_agent,
				AMD_AGENT_INFO_MEMORY_MAX_FREQUENCY,
				"AMD maximum memory clock",
			)?,
			product_name: agent_string::<64>(
				api,
				raw_agent,
				AMD_AGENT_INFO_PRODUCT_NAME,
				"AMD product name",
			)?,
			available_memory_bytes: agent_info(
				api,
				raw_agent,
				AMD_AGENT_INFO_MEMORY_AVAIL,
				"AMD available memory",
			)?,
			timestamp_frequency_hz: agent_info(
				api,
				raw_agent,
				AMD_AGENT_INFO_TIMESTAMP_FREQUENCY,
				"AMD timestamp frequency",
			)?,
			sdma_engine_count: agent_info(
				api,
				raw_agent,
				AMD_AGENT_INFO_NUM_SDMA_ENG,
				"AMD SDMA engine count",
			)?,
			xgmi_sdma_engine_count: agent_info(
				api,
				raw_agent,
				AMD_AGENT_INFO_NUM_SDMA_XGMI_ENG,
				"AMD XGMI SDMA engine count",
			)?,
			xcc_count: agent_info(api, raw_agent, AMD_AGENT_INFO_NUM_XCC, "AMD XCC count")?,
			maximum_scratch_bytes: agent_info(
				api,
				raw_agent,
				AMD_AGENT_INFO_SCRATCH_LIMIT_MAX,
				"AMD maximum scratch limit",
			)?,
			current_scratch_limit_bytes: agent_info(
				api,
				raw_agent,
				AMD_AGENT_INFO_SCRATCH_LIMIT_CURRENT,
				"AMD current scratch limit",
			)?,
		};
		(
			Some(driver_node_id),
			Some(PciAddress::from_hsa(domain, bdf_id)),
			Some(gpu),
		)
	} else {
		(None, None, None)
	};

	let raw_isas = iterate_isas(api, raw_agent)?;
	let mut isas = Vec::new();
	isas.try_reserve_exact(raw_isas.len())
		.map_err(|_| Error::AllocationFailed {
			field: "ISA descriptions",
			requested: raw_isas.len(),
		})?;
	for raw_isa in raw_isas {
		isas.push(discover_isa(api, raw_isa, device_type)?);
	}

	let raw_pool_handles = iterate_memory_pools(api, raw_agent)?;
	let mut memory_pools = Vec::new();
	memory_pools
		.try_reserve_exact(raw_pool_handles.len())
		.map_err(|_| Error::AllocationFailed {
			field: "memory-pool descriptions",
			requested: raw_pool_handles.len(),
		})?;
	let mut raw_pools = Vec::new();
	raw_pools
		.try_reserve_exact(raw_pool_handles.len())
		.map_err(|_| Error::AllocationFailed {
			field: "memory-pool handles",
			requested: raw_pool_handles.len(),
		})?;
	for pool in raw_pool_handles {
		memory_pools.push(discover_memory_pool(api, pool)?);
		raw_pools.push(RawPool { handle: pool });
	}

	let first_isa_wavefront_size = if feature_bits & AGENT_FEATURE_KERNEL_DISPATCH != 0 {
		Some(agent_info(
			api,
			raw_agent,
			AGENT_INFO_WAVEFRONT_SIZE,
			"HSA first-ISA wavefront size",
		)?)
	} else {
		None
	};

	Ok(DiscoveredAgent {
		runtime,
		raw_agent,
		raw_pools,
		description: AgentDescription {
			identity: AgentIdentity {
				name: agent_string::<64>(api, raw_agent, AGENT_INFO_NAME, "HSA agent name")?,
				vendor_name: agent_string::<64>(api, raw_agent, AGENT_INFO_VENDOR_NAME, "HSA agent vendor")?,
				uuid,
				numa_node_id: agent_info(api, raw_agent, AGENT_INFO_NODE, "HSA NUMA node ID")?,
				driver_node_id,
				pci_address,
			},
			device_type,
			profile,
			feature_bits,
			hsa_version_major: agent_info(
				api,
				raw_agent,
				AGENT_INFO_VERSION_MAJOR,
				"HSA agent major version",
			)?,
			hsa_version_minor: agent_info(
				api,
				raw_agent,
				AGENT_INFO_VERSION_MINOR,
				"HSA agent minor version",
			)?,
			first_isa_wavefront_size,
			queue,
			amd_gpu,
			isas,
			memory_pools,
		},
	})
}

fn validate_queue_capabilities(queue: QueueCapabilities) -> Result<()> {
	let valid = queue.minimum_packets != 0
		&& queue.minimum_packets.is_power_of_two()
		&& queue.maximum_packets != 0
		&& queue.maximum_packets.is_power_of_two()
		&& queue.minimum_packets <= queue.maximum_packets;
	if valid {
		Ok(())
	} else {
		Err(Error::InvalidQueueSize {
			requested: queue.minimum_packets,
			minimum: queue.minimum_packets,
			maximum: queue.maximum_packets,
			reason: "ROCr reported inconsistent queue limits",
		})
	}
}

fn discover_isa(api: &crate::loader::Api, isa: HsaIsa, device_type: DeviceType) -> Result<IsaDescription> {
	let name_length: u32 = isa_info(api, isa, ISA_INFO_NAME_LENGTH, "HSA ISA name length")?;
	let name_length = name_length as usize;
	if name_length > MAX_ISA_NAME_BYTES {
		return Err(Error::InvalidAttribute {
			field: "HSA_ISA_INFO_NAME_LENGTH",
			value: name_length as u64,
		});
	}
	let mut name_bytes = Vec::new();
	name_bytes
		.try_reserve_exact(name_length)
		.map_err(|_| Error::AllocationFailed {
			field: "HSA ISA name",
			requested: name_length,
		})?;
	name_bytes.resize(name_length, 0);
	if name_length != 0 {
		// SAFETY: `name_bytes` is exactly the length reported by ROCr and
		// remains writable for the call.
		let status =
			unsafe { (api.isa_get_info_alt)(isa, ISA_INFO_NAME, name_bytes.as_mut_ptr().cast::<c_void>()) };
		api.check("hsa_isa_get_info_alt(NAME)", status)?;
	}
	// The HSA header defines NAME_LENGTH as excluding NUL, while current ROCr
	// releases include one trailing NUL in the returned length. Accept either
	// representation, but reject embedded data after the first NUL.
	if let Some(nul) = name_bytes.iter().position(|byte| *byte == 0) {
		if name_bytes[nul..].iter().any(|byte| *byte != 0) {
			return Err(Error::InvalidAttribute {
				field: "HSA_ISA_INFO_NAME",
				value: nul as u64,
			});
		}
		name_bytes.truncate(nul);
	}
	let name = String::from_utf8(name_bytes).map_err(|_| Error::InvalidUtf8 {
		field: "HSA ISA name",
	})?;
	let amd_target = if device_type == DeviceType::Gpu {
		Some(IsaTarget::from_str(&name)?)
	} else {
		None
	};

	let machine_models: [u8; 2] = isa_info(api, isa, ISA_INFO_MACHINE_MODELS, "HSA ISA machine models")?;
	let profiles: [u8; 2] = isa_info(api, isa, ISA_INFO_PROFILES, "HSA ISA profiles")?;
	let grid: HsaDim3 = isa_info(
		api,
		isa,
		ISA_INFO_GRID_MAX_DIM,
		"HSA ISA maximum grid dimensions",
	)?;

	Ok(IsaDescription {
		name,
		amd_target,
		supports_small_machine_model: c_bool("HSA ISA small machine model", machine_models[0])?,
		supports_large_machine_model: c_bool("HSA ISA large machine model", machine_models[1])?,
		supports_base_profile: c_bool("HSA ISA base profile", profiles[0])?,
		supports_full_profile: c_bool("HSA ISA full profile", profiles[1])?,
		default_rounding_modes: RoundingModes::from_c_array(
			"HSA ISA default rounding modes",
			isa_info(
				api,
				isa,
				ISA_INFO_DEFAULT_FLOAT_ROUNDING_MODES,
				"HSA ISA default rounding modes",
			)?,
		)?,
		base_profile_rounding_modes: RoundingModes::from_c_array(
			"HSA ISA base-profile rounding modes",
			isa_info(
				api,
				isa,
				ISA_INFO_BASE_PROFILE_DEFAULT_FLOAT_ROUNDING_MODES,
				"HSA ISA base-profile rounding modes",
			)?,
		)?,
		fast_f16: c_bool(
			"HSA ISA fast f16",
			isa_info(api, isa, ISA_INFO_FAST_F16_OPERATION, "HSA ISA fast f16")?,
		)?,
		maximum_workgroup_dimensions: isa_info(
			api,
			isa,
			ISA_INFO_WORKGROUP_MAX_DIM,
			"HSA ISA maximum workgroup dimensions",
		)?,
		maximum_workgroup_size: isa_info(
			api,
			isa,
			ISA_INFO_WORKGROUP_MAX_SIZE,
			"HSA ISA maximum workgroup size",
		)?,
		maximum_grid_dimensions: [grid.x, grid.y, grid.z],
		maximum_grid_size: isa_info(
			api,
			isa,
			ISA_INFO_GRID_MAX_SIZE,
			"HSA ISA maximum grid size",
		)?,
		maximum_fbarriers_per_workgroup: isa_info(
			api,
			isa,
			ISA_INFO_FBARRIER_MAX_SIZE,
			"HSA ISA maximum fbarriers",
		)?,
	})
}

fn discover_memory_pool(api: &crate::loader::Api, pool: HsaMemoryPool) -> Result<MemoryPoolDescription> {
	let segment = MemorySegment::from_raw(pool_info(
		api,
		pool,
		AMD_MEMORY_POOL_INFO_SEGMENT,
		"AMD memory-pool segment",
	)?)?;
	let allocation_allowed = c_bool(
		"AMD memory-pool runtime allocation",
		pool_info(
			api,
			pool,
			AMD_MEMORY_POOL_INFO_RUNTIME_ALLOC_ALLOWED,
			"AMD memory-pool runtime allocation",
		)?,
	)?;
	let runtime_allocation = if allocation_allowed {
		Some(AllocationProperties {
			granule_bytes: pool_info(
				api,
				pool,
				AMD_MEMORY_POOL_INFO_RUNTIME_ALLOC_GRANULE,
				"AMD memory-pool allocation granule",
			)?,
			recommended_granule_bytes: pool_info(
				api,
				pool,
				AMD_MEMORY_POOL_INFO_RUNTIME_ALLOC_REC_GRANULE,
				"AMD memory-pool recommended allocation granule",
			)?,
			alignment_bytes: pool_info(
				api,
				pool,
				AMD_MEMORY_POOL_INFO_RUNTIME_ALLOC_ALIGNMENT,
				"AMD memory-pool allocation alignment",
			)?,
		})
	} else {
		None
	};

	Ok(MemoryPoolDescription {
		segment,
		location: pool_info_optional(
			api,
			pool,
			AMD_MEMORY_POOL_INFO_LOCATION,
			"AMD memory-pool location",
		)?
		.map(MemoryLocation::from_raw)
		.transpose()?,
		global_flags: if segment == MemorySegment::Global {
			Some(MemoryPoolFlags::from_raw(pool_info(
				api,
				pool,
				AMD_MEMORY_POOL_INFO_GLOBAL_FLAGS,
				"AMD memory-pool global flags",
			)?))
		} else {
			None
		},
		size_bytes: pool_info(api, pool, AMD_MEMORY_POOL_INFO_SIZE, "AMD memory-pool size")?,
		maximum_aggregate_allocation_bytes: pool_info_optional(
			api,
			pool,
			AMD_MEMORY_POOL_INFO_ALLOC_MAX_SIZE,
			"AMD memory-pool maximum allocation",
		)?,
		accessible_by_all_agents: c_bool(
			"AMD memory-pool all-agent accessibility",
			pool_info(
				api,
				pool,
				AMD_MEMORY_POOL_INFO_ACCESSIBLE_BY_ALL,
				"AMD memory-pool all-agent accessibility",
			)?,
		)?,
		runtime_allocation,
	})
}

fn system_info<T: Copy>(api: &crate::loader::Api, attribute: HsaInfo, operation: &'static str) -> Result<T> {
	let mut value = MaybeUninit::<T>::uninit();
	// SAFETY: every call site pairs the reviewed attribute with the exact
	// public-header output type, and the storage is writable for that type.
	let status = unsafe { (api.system_get_info)(attribute, value.as_mut_ptr().cast::<c_void>()) };
	api.check("hsa_system_get_info", status)
		.map_err(|error| annotate_operation(error, operation))?;
	// SAFETY: successful HSA information queries initialize the full value.
	Ok(unsafe { value.assume_init() })
}

fn agent_info<T: Copy>(
	api: &crate::loader::Api,
	agent: HsaAgent,
	attribute: HsaInfo,
	operation: &'static str,
) -> Result<T> {
	let mut value = MaybeUninit::<T>::uninit();
	// SAFETY: every call site pairs the reviewed attribute with its exact
	// public-header output type.
	let status = unsafe { (api.agent_get_info)(agent, attribute, value.as_mut_ptr().cast::<c_void>()) };
	api.check("hsa_agent_get_info", status)
		.map_err(|error| annotate_operation(error, operation))?;
	// SAFETY: successful HSA information queries initialize the full value.
	Ok(unsafe { value.assume_init() })
}

fn isa_info<T: Copy>(api: &crate::loader::Api, isa: HsaIsa, attribute: HsaInfo, operation: &'static str) -> Result<T> {
	let mut value = MaybeUninit::<T>::uninit();
	// SAFETY: every call site pairs the reviewed attribute with its exact
	// public-header output type.
	let status = unsafe { (api.isa_get_info_alt)(isa, attribute, value.as_mut_ptr().cast::<c_void>()) };
	api.check("hsa_isa_get_info_alt", status)
		.map_err(|error| annotate_operation(error, operation))?;
	// SAFETY: successful HSA information queries initialize the full value.
	Ok(unsafe { value.assume_init() })
}

fn pool_info<T: Copy>(
	api: &crate::loader::Api,
	pool: HsaMemoryPool,
	attribute: HsaInfo,
	operation: &'static str,
) -> Result<T> {
	let mut value = MaybeUninit::<T>::uninit();
	// SAFETY: every call site pairs the reviewed attribute with its exact
	// public-header output type.
	let status = unsafe { (api.amd_memory_pool_get_info)(pool, attribute, value.as_mut_ptr().cast::<c_void>()) };
	api.check("hsa_amd_memory_pool_get_info", status)
		.map_err(|error| annotate_operation(error, operation))?;
	// SAFETY: successful HSA information queries initialize the full value.
	Ok(unsafe { value.assume_init() })
}

fn pool_info_optional<T: Copy>(
	api: &crate::loader::Api,
	pool: HsaMemoryPool,
	attribute: HsaInfo,
	operation: &'static str,
) -> Result<Option<T>> {
	let mut value = MaybeUninit::<T>::uninit();
	// SAFETY: every call site pairs the reviewed attribute with its exact
	// public-header output type.
	let status = unsafe { (api.amd_memory_pool_get_info)(pool, attribute, value.as_mut_ptr().cast::<c_void>()) };
	if status == STATUS_ERROR_INVALID_ARGUMENT {
		return Ok(None);
	}
	api.check("hsa_amd_memory_pool_get_info", status)
		.map_err(|error| annotate_operation(error, operation))?;
	// SAFETY: successful HSA information queries initialize the full value.
	Ok(Some(unsafe { value.assume_init() }))
}

fn annotate_operation(error: Error, operation: &'static str) -> Error {
	match error {
		Error::Hsa {
			status, message, ..
		} => Error::Hsa {
			operation,
			status,
			message,
		},
		other => other,
	}
}

fn agent_string<const N: usize>(
	api: &crate::loader::Api,
	agent: HsaAgent,
	attribute: HsaInfo,
	field: &'static str,
) -> Result<String> {
	let bytes: [u8; N] = agent_info(api, agent, attribute, field)?;
	let Some(nul) = bytes.iter().position(|byte| *byte == 0) else {
		return Err(Error::InvalidAttribute {
			field,
			value: N as u64,
		});
	};
	std::str::from_utf8(&bytes[..nul])
		.map(str::to_owned)
		.map_err(|_| Error::InvalidUtf8 { field })
}

struct AgentCollector {
	values: Vec<HsaAgent>,
	panicked: bool,
}

unsafe extern "C" fn collect_agent(agent: HsaAgent, data: *mut c_void) -> HsaStatus {
	if data.is_null() {
		return STATUS_ERROR;
	}
	// SAFETY: `iterate_agents` passes the same live collector pointer supplied
	// by `iterate_agents` below, serially and only for that call's duration.
	let collector = unsafe { &mut *data.cast::<AgentCollector>() };
	match catch_unwind(AssertUnwindSafe(|| collector.values.push(agent))) {
		Ok(()) => STATUS_SUCCESS,
		Err(_) => {
			collector.panicked = true;
			STATUS_ERROR
		}
	}
}

fn iterate_agents(api: &crate::loader::Api) -> Result<Vec<HsaAgent>> {
	let mut collector = AgentCollector {
		values: Vec::new(),
		panicked: false,
	};
	// SAFETY: callback ABI matches the public header and data points to a live
	// collector for the synchronous traversal.
	let status = unsafe {
		(api.iterate_agents)(
			collect_agent,
			(&mut collector as *mut AgentCollector).cast::<c_void>(),
		)
	};
	if collector.panicked {
		return Err(Error::CallbackPanicked {
			operation: "hsa_iterate_agents",
		});
	}
	api.check("hsa_iterate_agents", status)?;
	Ok(collector.values)
}

struct IsaCollector {
	values: Vec<HsaIsa>,
	panicked: bool,
}

unsafe extern "C" fn collect_isa(isa: HsaIsa, data: *mut c_void) -> HsaStatus {
	if data.is_null() {
		return STATUS_ERROR;
	}
	// SAFETY: see `iterate_isas`; ROCr's traversal is synchronous.
	let collector = unsafe { &mut *data.cast::<IsaCollector>() };
	match catch_unwind(AssertUnwindSafe(|| collector.values.push(isa))) {
		Ok(()) => STATUS_SUCCESS,
		Err(_) => {
			collector.panicked = true;
			STATUS_ERROR
		}
	}
}

fn iterate_isas(api: &crate::loader::Api, agent: HsaAgent) -> Result<Vec<HsaIsa>> {
	let mut collector = IsaCollector {
		values: Vec::new(),
		panicked: false,
	};
	// SAFETY: callback and data satisfy the synchronous iterator contract.
	let status = unsafe {
		(api.agent_iterate_isas)(
			agent,
			collect_isa,
			(&mut collector as *mut IsaCollector).cast::<c_void>(),
		)
	};
	if collector.panicked {
		return Err(Error::CallbackPanicked {
			operation: "hsa_agent_iterate_isas",
		});
	}
	api.check("hsa_agent_iterate_isas", status)?;
	Ok(collector.values)
}

struct PoolCollector {
	values: Vec<HsaMemoryPool>,
	panicked: bool,
}

unsafe extern "C" fn collect_pool(pool: HsaMemoryPool, data: *mut c_void) -> HsaStatus {
	if data.is_null() {
		return STATUS_ERROR;
	}
	// SAFETY: see `iterate_memory_pools`; ROCr's traversal is synchronous.
	let collector = unsafe { &mut *data.cast::<PoolCollector>() };
	match catch_unwind(AssertUnwindSafe(|| collector.values.push(pool))) {
		Ok(()) => STATUS_SUCCESS,
		Err(_) => {
			collector.panicked = true;
			STATUS_ERROR
		}
	}
}

fn iterate_memory_pools(api: &crate::loader::Api, agent: HsaAgent) -> Result<Vec<HsaMemoryPool>> {
	let mut collector = PoolCollector {
		values: Vec::new(),
		panicked: false,
	};
	// SAFETY: callback and data satisfy the synchronous iterator contract.
	let status = unsafe {
		(api.amd_agent_iterate_memory_pools)(
			agent,
			collect_pool,
			(&mut collector as *mut PoolCollector).cast::<c_void>(),
		)
	};
	if collector.panicked {
		return Err(Error::CallbackPanicked {
			operation: "hsa_amd_agent_iterate_memory_pools",
		});
	}
	api.check("hsa_amd_agent_iterate_memory_pools", status)?;
	Ok(collector.values)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn rejects_inconsistent_queue_limits() {
		let error = validate_queue_capabilities(QueueCapabilities {
			minimum_packets: 128,
			maximum_packets: 64,
			advertised_kind: QueueKind::MultiProducer,
		})
		.unwrap_err();
		assert!(matches!(error, Error::InvalidQueueSize { .. }));
	}

	#[test]
	fn rejects_non_power_of_two_queue_limits() {
		assert!(
			validate_queue_capabilities(QueueCapabilities {
				minimum_packets: 96,
				maximum_packets: 256,
				advertised_kind: QueueKind::MultiProducer,
			})
			.is_err()
		);
	}
}
