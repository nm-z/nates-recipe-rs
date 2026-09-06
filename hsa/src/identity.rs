use std::{fmt, str::FromStr};

use crate::{Error, Result};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DeviceType {
	Cpu,
	Gpu,
	Dsp,
	Aie,
}

impl DeviceType {
	pub(crate) fn from_raw(raw: i32) -> Result<Self> {
		match raw {
			0 => Ok(Self::Cpu),
			1 => Ok(Self::Gpu),
			2 => Ok(Self::Dsp),
			3 => Ok(Self::Aie),
			_ => {
				Err(Error::InvalidAttribute {
					field: "HSA_AGENT_INFO_DEVICE",
					value: raw as u32 as u64,
				})
			}
		}
	}
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Profile {
	Base,
	Full,
}

impl Profile {
	pub(crate) fn from_raw(raw: i32, field: &'static str) -> Result<Self> {
		match raw {
			0 => Ok(Self::Base),
			1 => Ok(Self::Full),
			_ => {
				Err(Error::InvalidAttribute {
					field,
					value: raw as u32 as u64,
				})
			}
		}
	}

	pub(crate) const fn as_raw(self) -> i32 {
		match self {
			Self::Base => 0,
			Self::Full => 1,
		}
	}
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum QueueKind {
	MultiProducer,
	SingleProducer,
	Cooperative,
}

impl QueueKind {
	pub(crate) fn from_raw(raw: u32) -> Result<Self> {
		match raw {
			0 => Ok(Self::MultiProducer),
			1 => Ok(Self::SingleProducer),
			2 => Ok(Self::Cooperative),
			_ => {
				Err(Error::InvalidAttribute {
					field: "HSA_AGENT_INFO_QUEUE_TYPE",
					value: raw as u64,
				})
			}
		}
	}

	pub(crate) const fn as_raw(self) -> u32 {
		match self {
			Self::MultiProducer => 0,
			Self::SingleProducer => 1,
			Self::Cooperative => 2,
		}
	}
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MemorySegment {
	Global,
	ReadOnly,
	Private,
	Group,
}

impl MemorySegment {
	pub(crate) fn from_raw(raw: i32) -> Result<Self> {
		match raw {
			0 => Ok(Self::Global),
			1 => Ok(Self::ReadOnly),
			2 => Ok(Self::Private),
			3 => Ok(Self::Group),
			_ => {
				Err(Error::InvalidAttribute {
					field: "HSA_AMD_MEMORY_POOL_INFO_SEGMENT",
					value: raw as u32 as u64,
				})
			}
		}
	}
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MemoryLocation {
	Cpu,
	Gpu,
}

impl MemoryLocation {
	pub(crate) fn from_raw(raw: i32) -> Result<Self> {
		match raw {
			0 => Ok(Self::Cpu),
			1 => Ok(Self::Gpu),
			_ => {
				Err(Error::InvalidAttribute {
					field: "HSA_AMD_MEMORY_POOL_INFO_LOCATION",
					value: raw as u32 as u64,
				})
			}
		}
	}
}

/// ROCr global memory-pool flags. Unknown bits are retained so discovery never
/// silently rewrites a capability report from a newer runtime.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct MemoryPoolFlags(u32);

impl MemoryPoolFlags {
	pub const KERNARG_INITIALIZATION: u32 = crate::abi::MEMORY_POOL_GLOBAL_FLAG_KERNARG_INIT;
	pub const FINE_GRAINED: u32 = crate::abi::MEMORY_POOL_GLOBAL_FLAG_FINE_GRAINED;
	pub const COARSE_GRAINED: u32 = crate::abi::MEMORY_POOL_GLOBAL_FLAG_COARSE_GRAINED;
	pub const EXTENDED_SCOPE_FINE_GRAINED: u32 = crate::abi::MEMORY_POOL_GLOBAL_FLAG_EXTENDED_FINE_GRAINED;

	pub(crate) const fn from_raw(raw: u32) -> Self { Self(raw) }

	pub const fn bits(self) -> u32 { self.0 }

	pub const fn contains(self, flag: u32) -> bool { self.0 & flag == flag }
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct RoundingModes {
	pub default: bool,
	pub toward_zero: bool,
	pub nearest_even: bool,
}

impl RoundingModes {
	pub(crate) fn from_c_array(field: &'static str, values: [u8; 3]) -> Result<RoundingModes> {
		Ok(Self {
			default: c_bool(field, values[0])?,
			toward_zero: c_bool(field, values[1])?,
			nearest_even: c_bool(field, values[2])?,
		})
	}
}

pub(crate) fn c_bool(field: &'static str, raw: u8) -> Result<bool> {
	match raw {
		0 => Ok(false),
		1 => Ok(true),
		_ => {
			Err(Error::InvalidAttribute {
				field,
				value: raw as u64,
			})
		}
	}
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AgentUuidBody {
	Unavailable,
	Value([u8; 8]),
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct AgentUuid {
	raw: String,
	device_type: DeviceType,
	body: AgentUuidBody,
}

impl AgentUuid {
	pub fn as_str(&self) -> &str { &self.raw }

	pub const fn device_type(&self) -> DeviceType { self.device_type }

	pub const fn body(&self) -> AgentUuidBody { self.body }
}

impl FromStr for AgentUuid {
	type Err = Error;

	fn from_str(value: &str) -> Result<Self> {
		let Some((prefix, body)) = value.split_once('-') else {
			return Err(Error::InvalidIdentity {
				kind: "agent UUID",
				value: value.to_owned(),
				reason: "expected DEVICE-BODY",
			});
		};
		if body.contains('-') {
			return Err(Error::InvalidIdentity {
				kind: "agent UUID",
				value: value.to_owned(),
				reason: "contains an extra separator",
			});
		}

		let device_type = match prefix {
			"CPU" => DeviceType::Cpu,
			"GPU" => DeviceType::Gpu,
			"DSP" => DeviceType::Dsp,
			"AIE" => DeviceType::Aie,
			_ => {
				return Err(Error::InvalidIdentity {
					kind: "agent UUID",
					value: value.to_owned(),
					reason: "unknown device prefix",
				});
			}
		};

		let body = if body == "XX" {
			AgentUuidBody::Unavailable
		} else {
			if body.len() != 16 || !body.bytes().all(|byte| byte.is_ascii_hexdigit()) {
				return Err(Error::InvalidIdentity {
					kind: "agent UUID",
					value: value.to_owned(),
					reason: "body must be XX or exactly 16 hexadecimal digits",
				});
			}
			let mut bytes = [0_u8; 8];
			for (index, chunk) in body.as_bytes().as_chunks::<2>().0.iter().enumerate() {
				bytes[index] = hex_nibble(chunk[0]) * 16 + hex_nibble(chunk[1]);
			}
			AgentUuidBody::Value(bytes)
		};

		Ok(Self {
			raw: value.to_owned(),
			device_type,
			body,
		})
	}
}

impl fmt::Display for AgentUuid {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result { formatter.write_str(&self.raw) }
}

fn hex_nibble(value: u8) -> u8 {
	match value {
		b'0'..=b'9' => value - b'0',
		b'a'..=b'f' => value - b'a' + 10,
		b'A'..=b'F' => value - b'A' + 10,
		_ => unreachable!("validated as ASCII hex"),
	}
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct IsaFeature {
	name: String,
	enabled: bool,
}

impl IsaFeature {
	pub fn name(&self) -> &str { &self.name }

	pub const fn enabled(&self) -> bool { self.enabled }
}

/// Exact AMD HSA target identity, including ordered feature modifiers.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct IsaTarget {
	raw: String,
	architecture: String,
	features: Vec<IsaFeature>,
}

impl IsaTarget {
	pub fn as_str(&self) -> &str { &self.raw }

	pub fn architecture(&self) -> &str { &self.architecture }

	pub fn features(&self) -> &[IsaFeature] { &self.features }
}

impl FromStr for IsaTarget {
	type Err = Error;

	fn from_str(value: &str) -> Result<Self> {
		const PREFIX: &str = "amdgcn-amd-amdhsa--";
		let Some(tail) = value.strip_prefix(PREFIX) else {
			return Err(Error::InvalidIdentity {
				kind: "AMD ISA",
				value: value.to_owned(),
				reason: "expected amdgcn-amd-amdhsa-- prefix",
			});
		};
		let mut components = tail.split(':');
		let architecture = components.next().unwrap_or_default();
		let valid_architecture = architecture.strip_prefix("gfx").is_some_and(|suffix| {
			!suffix.is_empty()
				&& suffix
					.bytes()
					.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
		});
		if !valid_architecture {
			return Err(Error::InvalidIdentity {
				kind: "AMD ISA",
				value: value.to_owned(),
				reason: "architecture must be a lowercase gfx target identity",
			});
		}

		let mut features = Vec::new();
		for component in components {
			if component.len() < 2 {
				return Err(Error::InvalidIdentity {
					kind: "AMD ISA",
					value: value.to_owned(),
					reason: "feature must have a name followed by + or -",
				});
			}
			let (name, sign) = component.split_at(component.len() - 1);
			let valid_name = name
				.bytes()
				.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_');
			if !valid_name || name.is_empty() || (sign != "+" && sign != "-") {
				return Err(Error::InvalidIdentity {
					kind: "AMD ISA",
					value: value.to_owned(),
					reason: "malformed feature modifier",
				});
			}
			if features
				.iter()
				.any(|feature: &IsaFeature| feature.name == name)
			{
				return Err(Error::InvalidIdentity {
					kind: "AMD ISA",
					value: value.to_owned(),
					reason: "duplicate feature modifier",
				});
			}
			features.push(IsaFeature {
				name: name.to_owned(),
				enabled: sign == "+",
			});
		}

		Ok(Self {
			raw: value.to_owned(),
			architecture: architecture.to_owned(),
			features,
		})
	}
}

impl fmt::Display for IsaTarget {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result { formatter.write_str(&self.raw) }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PciAddress {
	pub domain: u32,
	pub bus: u8,
	pub device: u8,
	pub function: u8,
}

impl PciAddress {
	pub(crate) const fn from_hsa(domain: u32, bdf_id: u32) -> Self {
		Self {
			domain,
			bus: ((bdf_id >> 8) & 0xff) as u8,
			device: ((bdf_id >> 3) & 0x1f) as u8,
			function: (bdf_id & 0x7) as u8,
		}
	}
}

impl fmt::Display for PciAddress {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(
			formatter,
			"{:04x}:{:02x}:{:02x}.{}",
			self.domain, self.bus, self.device, self.function
		)
	}
}
