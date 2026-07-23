use crate::driver::Driver;
use crate::error::{CudaError, Result};
use crate::ffi::{
	CU_DEVICE_ATTRIBUTE_ASYNC_ENGINE_COUNT, CU_DEVICE_ATTRIBUTE_CLOCK_RATE,
	CU_DEVICE_ATTRIBUTE_COMPUTE_CAPABILITY_MAJOR, CU_DEVICE_ATTRIBUTE_COMPUTE_CAPABILITY_MINOR,
	CU_DEVICE_ATTRIBUTE_CONCURRENT_KERNELS, CU_DEVICE_ATTRIBUTE_GLOBAL_MEMORY_BUS_WIDTH,
	CU_DEVICE_ATTRIBUTE_MEMORY_CLOCK_RATE, CU_DEVICE_ATTRIBUTE_MULTIPROCESSOR_COUNT, CU_DEVICE_ATTRIBUTE_PCI_BUS_ID,
	CU_DEVICE_ATTRIBUTE_PCI_DEVICE_ID, CU_DEVICE_ATTRIBUTE_PCI_DOMAIN_ID, CUDA_ERROR_NOT_SUPPORTED, CuDevice, CuUuid,
	DriverCapabilities,
};
use core::ffi::{c_char, c_int};
use core::fmt;
use std::collections::BTreeMap;

const DEVICE_NAME_CAPACITY: usize = 256;
const PCI_BUS_ID_CAPACITY: usize = 32;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DeviceOrdinal(i32);

impl DeviceOrdinal {
	pub fn get(self) -> i32 {
		self.0
	}
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DeviceUuid([u8; 16]);

impl DeviceUuid {
	pub const fn from_bytes(bytes: [u8; 16]) -> Self {
		Self(bytes)
	}

	pub const fn as_bytes(&self) -> &[u8; 16] {
		&self.0
	}
}

impl fmt::Display for DeviceUuid {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let b = self.0;
		write!(
			f,
			"GPU-{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
			b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7], b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15],
		)
	}
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ComputeCapability {
	pub major: u32,
	pub minor: u32,
}

impl ComputeCapability {
	pub const fn new(major: u32, minor: u32) -> Self {
		Self { major, minor }
	}
}

impl fmt::Display for ComputeCapability {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "sm_{}{}", self.major, self.minor)
	}
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DriverVersion {
	raw: i32,
}

impl DriverVersion {
	pub fn from_raw(raw: i32) -> Result<Self> {
		if raw < 0 {
			return Err(CudaError::InvalidDriverValue {
				operation: "cuDriverGetVersion",
				detail: format!("negative version {raw}"),
			});
		}
		Ok(Self { raw })
	}

	pub fn new(major: u32, minor: u32) -> Result<Self> {
		let raw = major
			.checked_mul(1_000)
			.and_then(|value| value.checked_add(minor.checked_mul(10)?))
			.and_then(|value| i32::try_from(value).ok())
			.ok_or_else(|| CudaError::InvalidDriverValue {
				operation: "DriverVersion::new",
				detail: format!("{major}.{minor} is out of range"),
			})?;
		Ok(Self { raw })
	}

	pub const fn raw(self) -> i32 {
		self.raw
	}

	pub fn major(self) -> u32 {
		u32::try_from(self.raw / 1_000).unwrap_or_default()
	}

	pub fn minor(self) -> u32 {
		u32::try_from((self.raw % 1_000) / 10).unwrap_or_default()
	}
}

impl fmt::Display for DriverVersion {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}.{} ({})", self.major(), self.minor(), self.raw)
	}
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeviceAttributes {
	pub async_engine_count: u32,
	pub concurrent_kernels: bool,
	pub pci_domain_id: u32,
	pub pci_bus_id: u32,
	pub pci_device_id: u32,
	pub core_clock_khz: u32,
	pub memory_clock_khz: u32,
	pub global_memory_bus_width_bits: u32,
	pub multiprocessor_count: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeviceInfo {
	pub ordinal: DeviceOrdinal,
	pub uuid: DeviceUuid,
	pub name: String,
	pub compute_capability: ComputeCapability,
	/// Exact domain:bus:device.function string returned by the CUDA Driver.
	pub pci_bus_id: String,
	pub total_memory_bytes: u64,
	pub attributes: DeviceAttributes,
	pub(crate) handle: CuDevice,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Discovery {
	pub driver_version: DriverVersion,
	pub capabilities: DriverCapabilities,
	pub devices: Vec<DeviceInfo>,
}

impl Discovery {
	pub fn device(&self, ordinal: DeviceOrdinal) -> Option<&DeviceInfo> {
		self.devices.iter().find(|device| device.ordinal == ordinal)
	}
}

impl Driver {
	pub fn discover(&self) -> Result<Discovery> {
		self.check("cuInit", unsafe { (self.inner.api.init)(0) })?;

		let mut raw_version = 0;
		self.check("cuDriverGetVersion", unsafe {
			(self.inner.api.driver_get_version)(&raw mut raw_version)
		})?;
		let driver_version = DriverVersion::from_raw(raw_version)?;

		let mut raw_count = 0;
		self.check("cuDeviceGetCount", unsafe {
			(self.inner.api.device_get_count)(&raw mut raw_count)
		})?;
		let count = usize::try_from(raw_count).map_err(|_| CudaError::InvalidDriverValue {
			operation: "cuDeviceGetCount",
			detail: format!("negative device count {raw_count}"),
		})?;

		let mut devices = Vec::with_capacity(count);
		let mut uuids = BTreeMap::new();
		for raw_ordinal in 0..raw_count {
			let device = self.discover_device(raw_ordinal)?;
			if let Some(first) = uuids.insert(device.uuid, raw_ordinal) {
				return Err(CudaError::DuplicateDeviceUuid {
					first_ordinal: first,
					second_ordinal: raw_ordinal,
				});
			}
			devices.push(device);
		}

		Ok(Discovery {
			driver_version,
			capabilities: self.capabilities().clone(),
			devices,
		})
	}

	fn discover_device(&self, raw_ordinal: c_int) -> Result<DeviceInfo> {
		let mut handle = 0;
		self.check("cuDeviceGet", unsafe {
			(self.inner.api.device_get)(&raw mut handle, raw_ordinal)
		})?;

		let name = self.device_name(raw_ordinal, handle)?;
		let uuid = self.device_uuid(handle)?;
		let pci_bus_id = self.device_pci_bus_id(raw_ordinal, handle)?;
		let total_memory_bytes = self.device_memory(handle)?;
		let major = self.device_attribute(
			handle,
			CU_DEVICE_ATTRIBUTE_COMPUTE_CAPABILITY_MAJOR,
			"cuDeviceGetAttribute(COMPUTE_CAPABILITY_MAJOR)",
		)?;
		let minor = self.device_attribute(
			handle,
			CU_DEVICE_ATTRIBUTE_COMPUTE_CAPABILITY_MINOR,
			"cuDeviceGetAttribute(COMPUTE_CAPABILITY_MINOR)",
		)?;
		let concurrent_kernels = self.device_attribute(
			handle,
			CU_DEVICE_ATTRIBUTE_CONCURRENT_KERNELS,
			"cuDeviceGetAttribute(CONCURRENT_KERNELS)",
		)?;
		let concurrent_kernels = match concurrent_kernels {
			0 => false,
			1 => true,
			value => {
				return Err(CudaError::InvalidDriverValue {
					operation: "cuDeviceGetAttribute(CONCURRENT_KERNELS)",
					detail: format!("expected Boolean attribute, got {value}"),
				});
			}
		};
		let attributes = DeviceAttributes {
			async_engine_count: self.device_attribute(
				handle,
				CU_DEVICE_ATTRIBUTE_ASYNC_ENGINE_COUNT,
				"cuDeviceGetAttribute(ASYNC_ENGINE_COUNT)",
			)?,
			concurrent_kernels,
			pci_domain_id: self.device_attribute(
				handle,
				CU_DEVICE_ATTRIBUTE_PCI_DOMAIN_ID,
				"cuDeviceGetAttribute(PCI_DOMAIN_ID)",
			)?,
			pci_bus_id: self.device_attribute(
				handle,
				CU_DEVICE_ATTRIBUTE_PCI_BUS_ID,
				"cuDeviceGetAttribute(PCI_BUS_ID)",
			)?,
			pci_device_id: self.device_attribute(
				handle,
				CU_DEVICE_ATTRIBUTE_PCI_DEVICE_ID,
				"cuDeviceGetAttribute(PCI_DEVICE_ID)",
			)?,
			core_clock_khz: self.device_attribute(
				handle,
				CU_DEVICE_ATTRIBUTE_CLOCK_RATE,
				"cuDeviceGetAttribute(CLOCK_RATE)",
			)?,
			memory_clock_khz: self.device_attribute(
				handle,
				CU_DEVICE_ATTRIBUTE_MEMORY_CLOCK_RATE,
				"cuDeviceGetAttribute(MEMORY_CLOCK_RATE)",
			)?,
			global_memory_bus_width_bits: self.device_attribute(
				handle,
				CU_DEVICE_ATTRIBUTE_GLOBAL_MEMORY_BUS_WIDTH,
				"cuDeviceGetAttribute(GLOBAL_MEMORY_BUS_WIDTH)",
			)?,
			multiprocessor_count: self.device_attribute(
				handle,
				CU_DEVICE_ATTRIBUTE_MULTIPROCESSOR_COUNT,
				"cuDeviceGetAttribute(MULTIPROCESSOR_COUNT)",
			)?,
		};

		Ok(DeviceInfo {
			ordinal: DeviceOrdinal(raw_ordinal),
			uuid,
			name,
			compute_capability: ComputeCapability::new(major, minor),
			pci_bus_id,
			total_memory_bytes,
			attributes,
			handle,
		})
	}

	fn device_pci_bus_id(&self, ordinal: c_int, handle: CuDevice) -> Result<String> {
		let mut bytes = [0 as c_char; PCI_BUS_ID_CAPACITY];
		self.check("cuDeviceGetPCIBusId", unsafe {
			(self.inner.api.device_get_pci_bus_id)(
				bytes.as_mut_ptr(),
				c_int::try_from(bytes.len()).expect("PCI bus ID capacity fits c_int"),
				handle,
			)
		})?;
		let nul = bytes
			.iter()
			.position(|byte| *byte == 0)
			.ok_or_else(|| CudaError::InvalidDriverValue {
				operation: "cuDeviceGetPCIBusId",
				detail: format!("device {ordinal} bus ID is not NUL-terminated"),
			})?;
		let raw = unsafe { core::slice::from_raw_parts(bytes.as_ptr().cast::<u8>(), nul) };
		let value = core::str::from_utf8(raw).map_err(|error| CudaError::InvalidDriverValue {
			operation: "cuDeviceGetPCIBusId",
			detail: format!("device {ordinal} bus ID is not UTF-8: {error}"),
		})?;
		validate_pci_bus_id(value).map_err(|detail| CudaError::InvalidDriverValue {
			operation: "cuDeviceGetPCIBusId",
			detail: format!("device {ordinal} returned {value:?}: {detail}"),
		})?;
		Ok(value.to_owned())
	}

	fn device_name(&self, ordinal: c_int, handle: CuDevice) -> Result<String> {
		let mut bytes = [0 as c_char; DEVICE_NAME_CAPACITY];
		self.check("cuDeviceGetName", unsafe {
			(self.inner.api.device_get_name)(
				bytes.as_mut_ptr(),
				c_int::try_from(bytes.len()).expect("device name capacity fits c_int"),
				handle,
			)
		})?;
		let nul = bytes
			.iter()
			.position(|byte| *byte == 0)
			.ok_or_else(|| CudaError::InvalidDeviceName {
				ordinal,
				detail: format!("not NUL-terminated within {DEVICE_NAME_CAPACITY} bytes"),
			})?;
		let name_bytes = unsafe { core::slice::from_raw_parts(bytes.as_ptr().cast::<u8>(), nul) };
		let name = core::str::from_utf8(name_bytes)
			.map_err(|error| CudaError::InvalidDeviceName {
				ordinal,
				detail: error.to_string(),
			})?
			.trim()
			.to_owned();
		if name.is_empty() {
			return Err(CudaError::InvalidDeviceName {
				ordinal,
				detail: "empty name".to_owned(),
			});
		}
		Ok(name)
	}

	fn device_uuid(&self, handle: CuDevice) -> Result<DeviceUuid> {
		let mut uuid = CuUuid { bytes: [0; 16] };
		if let Some(get_uuid_v2) = self.inner.api.device_get_uuid_v2 {
			let status = unsafe { get_uuid_v2(&raw mut uuid, handle) };
			if status == crate::ffi::CUDA_SUCCESS {
				return Ok(DeviceUuid::from_bytes(uuid.bytes));
			}
			if status != CUDA_ERROR_NOT_SUPPORTED {
				self.check("cuDeviceGetUuid_v2", status)?;
			}
		}
		self.check("cuDeviceGetUuid", unsafe {
			(self.inner.api.device_get_uuid)(&raw mut uuid, handle)
		})?;
		Ok(DeviceUuid::from_bytes(uuid.bytes))
	}

	fn device_memory(&self, handle: CuDevice) -> Result<u64> {
		let mut bytes = 0usize;
		self.check("cuDeviceTotalMem_v2", unsafe {
			(self.inner.api.device_total_mem_v2)(&raw mut bytes, handle)
		})?;
		u64::try_from(bytes).map_err(|_| CudaError::InvalidDriverValue {
			operation: "cuDeviceTotalMem_v2",
			detail: format!("{bytes} does not fit u64"),
		})
	}

	fn device_attribute(&self, handle: CuDevice, attribute: c_int, operation: &'static str) -> Result<u32> {
		let mut value = 0;
		self.check(operation, unsafe {
			(self.inner.api.device_get_attribute)(&raw mut value, attribute, handle)
		})?;
		u32::try_from(value).map_err(|_| CudaError::InvalidDriverValue {
			operation,
			detail: format!("negative attribute value {value}"),
		})
	}
}

fn validate_pci_bus_id(value: &str) -> core::result::Result<(), &'static str> {
	let Some((domain, tail)) = value.split_once(':') else {
		return Err("missing PCI domain separator");
	};
	let Some((bus, slot)) = tail.split_once(':') else {
		return Err("missing PCI bus separator");
	};
	let Some((device, function)) = slot.split_once('.') else {
		return Err("missing PCI function separator");
	};
	let valid_hex = |component: &str, digits: usize| {
		component.len() == digits && component.bytes().all(|byte| byte.is_ascii_hexdigit())
	};
	if !valid_hex(domain, 4)
		|| !valid_hex(bus, 2)
		|| !valid_hex(device, 2)
		|| function.len() != 1
		|| !function.bytes().all(|byte| byte.is_ascii_digit())
	{
		return Err("expected dddd:bb:dd.f");
	}
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::ffi::{DriverSymbol, test_support};

	#[test]
	fn discovery_records_stable_device_identity() {
		let driver = Driver::from_test_api(test_support::api(false));
		let discovery = driver.discover().expect("fake discovery");
		assert_eq!(discovery.driver_version, DriverVersion::new(11, 4).unwrap());
		assert_eq!(discovery.devices.len(), 2);
		assert_eq!(discovery.devices[0].ordinal.get(), 0);
		assert_eq!(discovery.devices[1].ordinal.get(), 1);
		assert_eq!(discovery.devices[0].name, "Tesla K80");
		assert_eq!(
			discovery.devices[0].compute_capability,
			ComputeCapability::new(3, 7)
		);
		assert_eq!(discovery.devices[0].total_memory_bytes, 12_000_000_000);
		assert_eq!(discovery.devices[0].attributes.async_engine_count, 2);
		assert!(discovery.devices[0].attributes.concurrent_kernels);
		assert_eq!(discovery.devices[0].attributes.pci_bus_id, 3);
		assert_eq!(
			discovery.devices[0].attributes.global_memory_bus_width_bits,
			384
		);
		assert_eq!(
			discovery.devices[0].uuid.to_string(),
			"GPU-01000000-0000-0000-0000-000000000000"
		);
		assert!(
			!discovery
				.capabilities
				.supports(DriverSymbol::ModuleGetLoadingMode)
		);
	}

	#[test]
	fn compute_capability_uses_exact_sm_identity() {
		assert_eq!(ComputeCapability::new(5, 2).to_string(), "sm_52");
		assert_ne!(ComputeCapability::new(5, 2), ComputeCapability::new(8, 6));
	}
}
