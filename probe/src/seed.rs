use std::{
	collections::{BTreeMap, BTreeSet},
	path::Path,
};

use recipe_core::{ByteCount, BytesPerSecond, FlopsPerSecond, Label};

use crate::error::{ProbeError, ProbeResult};

pub const CONTRACT_SCHEMA: u32 = 1;
pub const CONTRACT_KIND: &str = "probe-seed-estimates";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeedEstimates {
	pub ethernet_rate: BytesPerSecond,
	pub disk_capacity: ByteCount,
	pub sata_rate: BytesPerSecond,
	pub gpu_memory_capacity: ByteCount,
	pub pcie_rate: BytesPerSecond,
	pub gpu_calculation_rate: FlopsPerSecond,
	pub gpu_memory_rate: BytesPerSecond,
	pub ram_capacity: ByteCount,
	pub ram_rate: BytesPerSecond,
	pub cpu_reference_rate: FlopsPerSecond,
	pub ram_transfer_rate: BytesPerSecond,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProbePolicy {
	pub discover_machines: bool,
	pub discover_devices: bool,
	pub discover_links: bool,
	pub benchmark_capacity: bool,
	pub benchmark_calculation: bool,
	pub benchmark_transfer: bool,
	pub require_measured_profile_for_prepare: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IdentityFacet {
	Machine,
	Device,
	Driver,
	RuntimeAbi,
	Firmware,
	Link,
	ArtifactToolchain,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeedDuplex {
	Full,
	Half,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransportSeed {
	pub name: Label,
	pub duplex: SeedDuplex,
}

/// Strict theoretical seed contract. It deliberately has no machine or device
/// inventory fields.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeedContract {
	pub schema: u32,
	pub estimates: SeedEstimates,
	pub reservation_per_storage_device: ByteCount,
	pub policy: ProbePolicy,
	pub invalidation: BTreeSet<IdentityFacet>,
	pub transports: Vec<TransportSeed>,
}

impl SeedContract {
	/// Reads a seed contract without modifying it.
	pub fn read(path: impl AsRef<Path>) -> ProbeResult<Self> {
		let path = path.as_ref();
		let input = std::fs::read_to_string(path).map_err(|error| ProbeError::io("read", path, error))?;
		Self::parse(&input)
	}

	pub fn parse(input: &str) -> ProbeResult<Self> {
		let assignments = parse_assignments(input)?;
		let schema = parse_u32(required(&assignments, "schema")?)?;
		if schema != CONTRACT_SCHEMA {
			return Err(ProbeError::contract(
				None,
				format!("unsupported schema {schema}; expected {CONTRACT_SCHEMA}"),
			));
		}
		let kind = parse_string(required(&assignments, "kind")?)?;
		if kind != CONTRACT_KIND {
			return Err(ProbeError::contract(
				None,
				format!("kind must be {CONTRACT_KIND:?}, got {kind:?}"),
			));
		}

		let estimates = SeedEstimates {
			ethernet_rate: rate(&assignments, "estimates.ethernet_bytes_per_second")?,
			disk_capacity: bytes(&assignments, "estimates.disk_bytes")?,
			sata_rate: rate(&assignments, "estimates.sata_bytes_per_second")?,
			gpu_memory_capacity: bytes(&assignments, "estimates.gpu_vram_bytes")?,
			pcie_rate: rate(&assignments, "estimates.pcie_bytes_per_second")?,
			gpu_calculation_rate: flop_rate(&assignments, "estimates.gpu_flops_per_second")?,
			gpu_memory_rate: rate(&assignments, "estimates.gpu_transfer_bytes_per_second")?,
			ram_capacity: bytes(&assignments, "estimates.ram_bytes")?,
			ram_rate: rate(&assignments, "estimates.ddr_bytes_per_second")?,
			cpu_reference_rate: flop_rate(&assignments, "estimates.cpu_reference_flops_per_second")?,
			ram_transfer_rate: rate(&assignments, "estimates.ram_transfer_bytes_per_second")?,
		};
		let reservation_per_storage_device = bytes(&assignments, "reservation.bytes_per_storage_device")?;
		if reservation_per_storage_device != ByteCount::new(1_000_000_000) {
			return Err(ProbeError::contract(
				None,
				"reservation.bytes_per_storage_device must be exactly 1_000_000_000",
			));
		}

		let command = parse_string(required(&assignments, "probe.command")?)?;
		let environment = parse_string(required(&assignments, "probe.environment")?)?;
		if command != "recipe probe" || environment != "bare-metal" {
			return Err(ProbeError::contract(
				None,
				"probe command/environment must be `recipe probe` on `bare-metal`",
			));
		}
		let policy = ProbePolicy {
			discover_machines: boolean(&assignments, "probe.discover_machines")?,
			discover_devices: boolean(&assignments, "probe.discover_devices")?,
			discover_links: boolean(&assignments, "probe.discover_links")?,
			benchmark_capacity: boolean(&assignments, "probe.benchmark_capacity")?,
			benchmark_calculation: boolean(&assignments, "probe.benchmark_calculation")?,
			benchmark_transfer: boolean(&assignments, "probe.benchmark_transfer")?,
			require_measured_profile_for_prepare: boolean(&assignments, "probe.require_measured_profile_for_prepare")?,
		};
		if ![
			policy.discover_machines,
			policy.discover_devices,
			policy.discover_links,
			policy.benchmark_capacity,
			policy.benchmark_calculation,
			policy.benchmark_transfer,
			policy.require_measured_profile_for_prepare,
		]
		.into_iter()
		.all(|enabled| enabled)
		{
			return Err(ProbeError::contract(
				None,
				"all discovery, benchmark, and measured-profile gates must be enabled",
			));
		}

		let invalidation = parse_string_array(required(&assignments, "probe.invalidate_on")?)?
			.into_iter()
			.map(|value| {
				match value.as_str() {
					"machine" => Ok(IdentityFacet::Machine),
					"device" => Ok(IdentityFacet::Device),
					"driver" => Ok(IdentityFacet::Driver),
					"runtime-abi" => Ok(IdentityFacet::RuntimeAbi),
					"firmware" => Ok(IdentityFacet::Firmware),
					"link" => Ok(IdentityFacet::Link),
					"artifact-toolchain" => Ok(IdentityFacet::ArtifactToolchain),
					other => {
						Err(ProbeError::contract(
							None,
							format!("unknown cache invalidation facet {other:?}"),
						))
					}
				}
			})
			.collect::<ProbeResult<BTreeSet<_>>>()?;
		let all_facets = [
			IdentityFacet::Machine,
			IdentityFacet::Device,
			IdentityFacet::Driver,
			IdentityFacet::RuntimeAbi,
			IdentityFacet::Firmware,
			IdentityFacet::Link,
			IdentityFacet::ArtifactToolchain,
		]
		.into_iter()
		.collect::<BTreeSet<_>>();
		if invalidation != all_facets {
			return Err(ProbeError::contract(
				None,
				"cache invalidation must include every machine, device, driver, runtime, firmware, link, and toolchain facet",
			));
		}

		let mut transport_names = BTreeSet::new();
		for key in assignments.keys() {
			if let Some(rest) = key.strip_prefix("transport.")
				&& let Some((name, _field)) = rest.split_once('.')
			{
				transport_names.insert(name.to_owned());
			}
		}
		if transport_names.is_empty() {
			return Err(ProbeError::contract(
				None,
				"at least one transport seed is required",
			));
		}
		let mut transports = Vec::new();
		for name in transport_names {
			let prefix = format!("transport.{name}");
			let directions = parse_string(required(&assignments, &format!("{prefix}.directions"))?)?;
			let issue = parse_string(required(&assignments, &format!("{prefix}.issue"))?)?;
			if directions != "both" || issue != "async" {
				return Err(ProbeError::contract(
					None,
					format!("{prefix} must be bidirectional and asynchronous"),
				));
			}
			let duplex = match parse_string(required(&assignments, &format!("{prefix}.duplex"))?)?.as_str() {
				"full" => SeedDuplex::Full,
				"half" => SeedDuplex::Half,
				other => {
					return Err(ProbeError::contract(
						None,
						format!("{prefix}.duplex has invalid value {other:?}"),
					));
				}
			};
			transports.push(TransportSeed {
				name: Label::new(name).map_err(|error| ProbeError::contract(None, error.to_string()))?,
				duplex,
			});
		}

		reject_unknown_fields(&assignments)?;
		Ok(Self {
			schema,
			estimates,
			reservation_per_storage_device,
			policy,
			invalidation,
			transports,
		})
	}
}

#[derive(Clone, Debug)]
struct Assignment {
	line: usize,
	value: String,
}

fn parse_assignments(input: &str) -> ProbeResult<BTreeMap<String, Assignment>> {
	let mut result = BTreeMap::new();
	let mut section = String::new();
	let mut pending: Option<(usize, String, String)> = None;
	for (zero_index, raw) in input.lines().enumerate() {
		let line_number = zero_index + 1;
		let line = strip_comment(raw).trim().to_owned();
		if line.is_empty() {
			continue;
		}
		if let Some((start, key, mut value)) = pending.take() {
			value.push(' ');
			value.push_str(&line);
			if value.contains(']') {
				insert_assignment(&mut result, key, start, value)?;
			} else {
				pending = Some((start, key, value));
			}
			continue;
		}
		if line.starts_with('[') && line.ends_with(']') {
			section = line[1..line.len() - 1].trim().to_owned();
			if section.is_empty() {
				return Err(ProbeError::contract(Some(line_number), "empty table name"));
			}
			continue;
		}
		let Some((key, value)) = line.split_once('=') else {
			return Err(ProbeError::contract(
				Some(line_number),
				"expected key = value",
			));
		};
		let key = key.trim();
		if key.is_empty() {
			return Err(ProbeError::contract(Some(line_number), "empty key"));
		}
		let full_key = if section.is_empty() {
			key.to_owned()
		} else {
			format!("{section}.{key}")
		};
		let value = value.trim().to_owned();
		if value.starts_with('[') && !value.contains(']') {
			pending = Some((line_number, full_key, value));
		} else {
			insert_assignment(&mut result, full_key, line_number, value)?;
		}
	}
	if let Some((line, _, _)) = pending {
		return Err(ProbeError::contract(Some(line), "unterminated array"));
	}
	Ok(result)
}

fn insert_assignment(result: &mut BTreeMap<String, Assignment>, key: String, line: usize, value: String) -> ProbeResult<()> {
	if result
		.insert(key.clone(), Assignment { line, value })
		.is_some()
	{
		return Err(ProbeError::contract(
			Some(line),
			format!("duplicate key {key}"),
		));
	}
	Ok(())
}

fn strip_comment(line: &str) -> &str {
	let mut quoted = false;
	for (index, byte) in line.bytes().enumerate() {
		if byte == b'"' {
			quoted = !quoted;
		} else if byte == b'#' && !quoted {
			return &line[..index];
		}
	}
	line
}

fn required<'a>(values: &'a BTreeMap<String, Assignment>, key: &str) -> ProbeResult<&'a Assignment> {
	values.get(key)
		.ok_or_else(|| ProbeError::contract(None, format!("missing required key {key}")))
}

fn parse_string(value: &Assignment) -> ProbeResult<String> {
	let raw = value.value.trim();
	if raw.len() >= 2 && raw.starts_with('"') && raw.ends_with('"') {
		Ok(raw[1..raw.len() - 1].to_owned())
	} else {
		Err(ProbeError::contract(
			Some(value.line),
			format!("expected quoted string, got {raw}"),
		))
	}
}

fn parse_string_array(value: &Assignment) -> ProbeResult<Vec<String>> {
	let raw = value.value.trim();
	if !raw.starts_with('[') || !raw.ends_with(']') {
		return Err(ProbeError::contract(
			Some(value.line),
			"expected string array",
		));
	}
	let inside = &raw[1..raw.len() - 1];
	inside.split(',')
		.map(str::trim)
		.filter(|entry| !entry.is_empty())
		.map(|entry| {
			parse_string(&Assignment {
				line: value.line,
				value: entry.to_owned(),
			})
		})
		.collect()
}

fn parse_u64(value: &Assignment) -> ProbeResult<u64> {
	value.value.replace('_', "").parse().map_err(|error| {
		ProbeError::contract(
			Some(value.line),
			format!("expected unsigned integer: {error}"),
		)
	})
}

fn parse_u32(value: &Assignment) -> ProbeResult<u32> {
	u32::try_from(parse_u64(value)?).map_err(|_| {
		ProbeError::contract(
			Some(value.line),
			"integer does not fit unsigned 32-bit value",
		)
	})
}

fn boolean(values: &BTreeMap<String, Assignment>, key: &str) -> ProbeResult<bool> {
	let value = required(values, key)?;
	match value.value.as_str() {
		"true" => Ok(true),
		"false" => Ok(false),
		_ => {
			Err(ProbeError::contract(
				Some(value.line),
				format!("{key} must be true or false"),
			))
		}
	}
}

fn bytes(values: &BTreeMap<String, Assignment>, key: &str) -> ProbeResult<ByteCount> { Ok(ByteCount::new(parse_u64(required(values, key)?)?)) }

fn rate(values: &BTreeMap<String, Assignment>, key: &str) -> ProbeResult<BytesPerSecond> { BytesPerSecond::new(parse_u64(required(values, key)?)?).map_err(|error| ProbeError::contract(None, format!("{key}: {error}"))) }

fn flop_rate(values: &BTreeMap<String, Assignment>, key: &str) -> ProbeResult<FlopsPerSecond> { FlopsPerSecond::new(parse_u64(required(values, key)?)?).map_err(|error| ProbeError::contract(None, format!("{key}: {error}"))) }

fn reject_unknown_fields(values: &BTreeMap<String, Assignment>) -> ProbeResult<()> {
	const EXACT_KEYS: &[&str] = &[
		"schema",
		"kind",
		"estimates.ethernet_bytes_per_second",
		"estimates.disk_bytes",
		"estimates.sata_bytes_per_second",
		"estimates.gpu_vram_bytes",
		"estimates.pcie_bytes_per_second",
		"estimates.gpu_flops_per_second",
		"estimates.gpu_transfer_bytes_per_second",
		"estimates.ram_bytes",
		"estimates.ddr_bytes_per_second",
		"estimates.cpu_reference_flops_per_second",
		"estimates.ram_transfer_bytes_per_second",
		"reservation.bytes_per_storage_device",
		"probe.command",
		"probe.environment",
		"probe.discover_machines",
		"probe.discover_devices",
		"probe.discover_links",
		"probe.benchmark_capacity",
		"probe.benchmark_calculation",
		"probe.benchmark_transfer",
		"probe.require_measured_profile_for_prepare",
		"probe.invalidate_on",
	];
	for (key, assignment) in values {
		if EXACT_KEYS.contains(&key.as_str()) {
			continue;
		}
		let transport_field = key
			.strip_prefix("transport.")
			.and_then(|rest| rest.rsplit_once('.'))
			.is_some_and(|(name, field)| !name.is_empty() && !name.contains('.') && matches!(field, "directions" | "duplex" | "issue"));
		if transport_field {
			continue;
		}
		return Err(ProbeError::contract(
			Some(assignment.line),
			format!("unsupported field {key}; machine, device, link, and production-rate inventory is discovered automatically"),
		));
	}
	Ok(())
}
