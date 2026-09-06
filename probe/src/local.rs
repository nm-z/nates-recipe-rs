use std::{
	collections::{BTreeMap, BTreeSet},
	fs::{self, File, OpenOptions},
	hint::black_box,
	io::{Read, Seek, SeekFrom, Write},
	os::unix::fs::{MetadataExt as _, OpenOptionsExt as _},
	path::{Path, PathBuf},
	sync::atomic::{AtomicU64, Ordering},
	time::Instant,
};

use recipe_core::{ByteCount, BytesPerSecond, Label, Property, PropertyProvenance, TransferLaneCount, TransportKind};

use crate::{
	error::{ProbeError, ProbeResult},
	model::{BoundedBenchmarkPlan, HostBenchmarkIo, HostDiscovery, HostInventory, LinkDuplex, MachineFingerprint, NetworkInterface, RamDomain, RamMeasurement, StorageDomain, StorageMeasurement},
};

/// Linux bare-metal discovery using procfs and sysfs only.
#[derive(Clone, Debug)]
pub struct LocalSystemDiscovery {
	proc_root: PathBuf,
	sys_root: PathBuf,
	etc_root: PathBuf,
	benchmark_roots: Vec<PathBuf>,
}

impl Default for LocalSystemDiscovery {
	fn default() -> Self {
		Self {
			proc_root: PathBuf::from("/proc"),
			sys_root: PathBuf::from("/sys"),
			etc_root: PathBuf::from("/etc"),
			benchmark_roots: Vec::new(),
		}
	}
}

impl HostDiscovery for LocalSystemDiscovery {
	fn discover_host(&self) -> ProbeResult<HostInventory> {
		let hostname = read_trimmed(self.proc_root.join("sys/kernel/hostname"))?;
		let stable_id = read_first_available(&[
			self.etc_root.join("machine-id"),
			self.sys_root.join("class/dmi/id/product_uuid"),
		])?;
		let runtime_abi = read_trimmed(self.proc_root.join("sys/kernel/osrelease"))?;
		let firmware = join_available(
			&[
				self.sys_root.join("class/dmi/id/bios_vendor"),
				self.sys_root.join("class/dmi/id/bios_version"),
				self.sys_root.join("class/dmi/id/bios_date"),
			],
			"|",
		)
		.unwrap_or_else(|| "firmware-unreported".to_owned());
		let ram = self.discover_ram()?;
		if ram.is_empty() {
			return Err(ProbeError::Discovery(
				"no host memory domain was discovered".to_owned(),
			));
		}
		let storage = self.discover_storage(&ram[0].key)?;
		let network = self.discover_network()?;
		Ok(HostInventory {
			machine: MachineFingerprint {
				hostname: label(hostname)?,
				stable_id: label(stable_id)?,
				runtime_abi: label(runtime_abi)?,
				firmware: label(firmware)?,
			},
			ram,
			storage,
			network,
		})
	}
}

impl LocalSystemDiscovery {
	/// Prefer these already-created writable directories when selecting the
	/// benchmark location for their underlying physical disks.
	///
	/// The roots select only where a bounded temporary file may be placed.
	/// They do not declare a disk, capacity, transport, identity, or rate.
	#[must_use]
	pub fn with_benchmark_roots(benchmark_roots: Vec<PathBuf>) -> Self {
		Self {
			benchmark_roots,
			..Self::default()
		}
	}

	fn discover_ram(&self) -> ProbeResult<Vec<RamDomain>> {
		let node_root = self.sys_root.join("devices/system/node");
		let mut domains = Vec::new();
		if let Ok(entries) = fs::read_dir(&node_root) {
			let mut paths = entries
				.filter_map(Result::ok)
				.map(|entry| entry.path())
				.filter(|path| {
					path.file_name()
						.and_then(|name| name.to_str())
						.is_some_and(|name| {
							name.strip_prefix("node")
								.is_some_and(|suffix| suffix.chars().all(|ch| ch.is_ascii_digit()))
						})
				})
				.collect::<Vec<_>>();
			paths.sort();
			for path in paths {
				let name = path
					.file_name()
					.and_then(|value| value.to_str())
					.ok_or_else(|| ProbeError::Discovery("invalid memory-node name".to_owned()))?;
				let bytes = parse_memtotal(&fs::read_to_string(path.join("meminfo")).map_err(|error| ProbeError::io("read", path.join("meminfo"), error))?)?;
				domains.push(RamDomain {
					key: label(name)?,
					capacity_hint: ByteCount::new(bytes),
					link_identity: label(format!("memory-link:{name}"))?,
					maximum_inflight_transfers: TransferLaneCount::new(1).map_err(|error| ProbeError::Discovery(error.to_string()))?,
				});
			}
		}
		if domains.is_empty() {
			let path = self.proc_root.join("meminfo");
			let bytes = parse_memtotal(&fs::read_to_string(&path).map_err(|error| ProbeError::io("read", &path, error))?)?;
			domains.push(RamDomain {
				key: label("memory0")?,
				capacity_hint: ByteCount::new(bytes),
				link_identity: label("memory-link:memory0")?,
				maximum_inflight_transfers: TransferLaneCount::new(1).map_err(|error| ProbeError::Discovery(error.to_string()))?,
			});
		}
		Ok(domains)
	}

	fn discover_storage(&self, host_memory_key: &Label) -> ProbeResult<Vec<StorageDomain>> {
		let mountinfo_path = self.proc_root.join("self/mountinfo");
		let mountinfo = fs::read_to_string(&mountinfo_path).map_err(|error| ProbeError::io("read", &mountinfo_path, error))?;
		let mut groups = BTreeMap::<String, PhysicalStorage>::new();
		for line in mountinfo.lines() {
			let fields = line.split_whitespace().collect::<Vec<_>>();
			let Some(separator) = fields.iter().position(|field| *field == "-") else {
				continue;
			};
			if separator < 5 || fields.len() <= separator + 2 {
				continue;
			}
			let major_minor = fields[2];
			let sys_device = self.sys_root.join("dev/block").join(major_minor);
			if !sys_device.exists() {
				continue;
			}
			let mounted_block = fs::canonicalize(&sys_device).map_err(|error| ProbeError::io("canonicalize", &sys_device, error))?;
			let physical = physical_block_device(&mounted_block)?;
			let identity = physical.to_string_lossy().into_owned();
			let sectors = read_trimmed(physical.join("size"))?
				.parse::<u64>()
				.map_err(|error| {
					ProbeError::Discovery(format!(
						"invalid sector count for {}: {error}",
						physical.display()
					))
				})?;
			let capacity = sectors.checked_mul(512).ok_or_else(|| {
				ProbeError::Discovery(format!(
					"storage capacity overflow for {}",
					physical.display()
				))
			})?;
			if capacity == 0 {
				continue;
			}
			let mount = PathBuf::from(unescape_mount(fields[4]));
			let source = fields[separator + 2];
			let device_name = physical
				.file_name()
				.and_then(|name| name.to_str())
				.unwrap_or(source);
			let firmware = join_available(
				&[
					physical.join("device/model"),
					physical.join("device/rev"),
					physical.join("device/firmware_rev"),
				],
				"|",
			)
			.unwrap_or_else(|| "storage-firmware-unreported".to_owned());
			let transport_kind = if identity.contains("/nvme") {
				TransportKind::Nvme
			} else if identity.contains("/sas") {
				TransportKind::Sas
			} else {
				TransportKind::Sata
			};
			let full_duplex = matches!(transport_kind, TransportKind::Nvme | TransportKind::Sas);
			let physical_major_minor = read_trimmed(physical.join("dev"))?;
			let group = groups.entry(identity.clone()).or_insert_with(|| {
				PhysicalStorage {
					major_minor: physical_major_minor,
					name: device_name.to_owned(),
					identity: identity.clone(),
					firmware,
					capacity: ByteCount::new(capacity),
					transport_kind,
					duplex: if full_duplex {
						LinkDuplex::Full
					} else {
						LinkDuplex::Half
					},
					mounted_devices: BTreeSet::new(),
					mounts: BTreeSet::new(),
				}
			});
			if group.capacity != ByteCount::new(capacity) || group.transport_kind != transport_kind {
				return Err(ProbeError::Discovery(format!(
					"inconsistent physical storage identity {identity}"
				)));
			}
			group.mounted_devices.insert(major_minor.to_owned());
			group.mounts.insert(mount);
		}
		let mut result = Vec::with_capacity(groups.len());
		for group in groups.into_values() {
			let benchmark_root = select_storage_benchmark_root(&group, &self.benchmark_roots)?;
			result.push(StorageDomain {
				key: label(format!("block:{}", group.major_minor))?,
				name: label(group.name)?,
				benchmark_root,
				capacity_hint: group.capacity,
				host_memory_key: host_memory_key.clone(),
				driver: label(group.identity.clone())?,
				firmware: label(group.firmware)?,
				link_identity: label(format!("storage-link:{}", group.identity))?,
				transport_kind: group.transport_kind,
				duplex: group.duplex,
				maximum_concurrent_reads: TransferLaneCount::new(1).map_err(|error| ProbeError::Discovery(error.to_string()))?,
				maximum_concurrent_writes: TransferLaneCount::new(1).map_err(|error| ProbeError::Discovery(error.to_string()))?,
				asynchronous_submission: true,
			});
		}
		result.sort_by(|left, right| left.key.cmp(&right.key));
		Ok(result)
	}

	fn discover_network(&self) -> ProbeResult<Vec<NetworkInterface>> {
		let root = self.sys_root.join("class/net");
		let entries = fs::read_dir(&root).map_err(|error| ProbeError::io("read directory", &root, error))?;
		let mut paths = entries
			.filter_map(Result::ok)
			.map(|entry| entry.path())
			.filter(|path| path.file_name().and_then(|name| name.to_str()) != Some("lo"))
			.collect::<Vec<_>>();
		paths.sort();
		let mut result = Vec::new();
		for path in paths {
			let name = path
				.file_name()
				.and_then(|value| value.to_str())
				.ok_or_else(|| ProbeError::Discovery("invalid network-interface name".to_owned()))?;
			let address = read_trimmed(path.join("address"))?;
			let ifindex = read_trimmed(path.join("ifindex"))?;
			let device = path.join("device");
			let driver = fs::canonicalize(device.join("driver"))
				.map(|path| path.to_string_lossy().into_owned())
				.unwrap_or_else(|_| "network-driver-unreported".to_owned());
			let firmware = join_available(
				&[device.join("firmware_version"), device.join("uevent")],
				"|",
			)
			.unwrap_or_else(|| "network-firmware-unreported".to_owned());
			let duplex = if path.join("wireless").exists() {
				LinkDuplex::Half
			} else {
				LinkDuplex::Full
			};
			result.push(NetworkInterface {
				key: label(format!("net:{ifindex}:{address}"))?,
				name: label(name)?,
				address: label(&address)?,
				driver: label(driver)?,
				firmware: label(firmware)?,
				link_identity: label(format!("network-link:{ifindex}:{address}"))?,
				transport_kind: if path.join("wireless").exists() {
					TransportKind::Wlan
				} else {
					TransportKind::Ethernet
				},
				duplex,
				asynchronous_submission: true,
			});
		}
		Ok(result)
	}
}

#[derive(Clone, Debug)]
struct PhysicalStorage {
	major_minor: String,
	name: String,
	identity: String,
	firmware: String,
	capacity: ByteCount,
	transport_kind: TransportKind,
	duplex: LinkDuplex,
	mounted_devices: BTreeSet<String>,
	mounts: BTreeSet<PathBuf>,
}

fn physical_block_device(mounted: &Path) -> ProbeResult<PathBuf> {
	let mut physical = mounted.to_path_buf();
	while physical.join("partition").exists() {
		physical = physical
			.parent()
			.ok_or_else(|| {
				ProbeError::Discovery(format!(
					"partition {} has no parent block device",
					mounted.display()
				))
			})?
			.to_path_buf();
	}
	if !physical.join("size").exists() || !physical.join("dev").exists() {
		return Err(ProbeError::Discovery(format!(
			"physical block identity {} lacks size or device numbers",
			physical.display()
		)));
	}
	Ok(physical)
}

fn select_storage_benchmark_root(storage: &PhysicalStorage, configured: &[PathBuf]) -> ProbeResult<PathBuf> {
	let mut candidates = configured.to_vec();
	if let Some(home) = std::env::var_os("HOME") {
		candidates.push(PathBuf::from(home));
	}
	if let Ok(current) = std::env::current_dir() {
		candidates.push(current);
	}
	candidates.extend(storage.mounts.iter().cloned());

	let mut seen = BTreeSet::new();
	for candidate in candidates {
		let canonical = match fs::canonicalize(&candidate) {
			Ok(canonical) => canonical,
			Err(_) => continue,
		};
		if !seen.insert(canonical.clone()) {
			continue;
		}
		let metadata = match fs::metadata(&canonical) {
			Ok(metadata) if metadata.is_dir() => metadata,
			Ok(_) | Err(_) => continue,
		};
		let device = metadata.dev();
		let major_minor = format!(
			"{}:{}",
			rustix::fs::major(device),
			rustix::fs::minor(device)
		);
		if !storage.mounted_devices.contains(&major_minor) {
			continue;
		}
		if directory_accepts_probe_file(&canonical)? {
			return Ok(canonical);
		}
	}
	Err(ProbeError::Discovery(format!(
		"physical disk {} has no user-writable benchmark directory on any mounted filesystem",
		storage.identity
	)))
}

fn directory_accepts_probe_file(directory: &Path) -> ProbeResult<bool> {
	static NEXT: AtomicU64 = AtomicU64::new(1);
	for _ in 0..64 {
		let nonce = NEXT.fetch_add(1, Ordering::Relaxed);
		let path = directory.join(format!(
			".recipe-probe-access-{}-{nonce}.tmp",
			std::process::id()
		));
		match OpenOptions::new()
			.write(true)
			.create_new(true)
			.mode(0o600)
			.open(&path)
		{
			Ok(file) => {
				drop(file);
				fs::remove_file(&path).map_err(|error| ProbeError::io("remove access probe", &path, error))?;
				return Ok(true);
			}
			Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
			Err(error)
				if matches!(
					error.kind(),
					std::io::ErrorKind::PermissionDenied | std::io::ErrorKind::ReadOnlyFilesystem
				) =>
			{
				return Ok(false);
			}
			Err(error) => {
				return Err(ProbeError::io(
					"test storage benchmark directory",
					&path,
					error,
				));
			}
		}
	}
	Err(ProbeError::Discovery(format!(
		"could not allocate an access-test filename in {}",
		directory.display()
	)))
}

/// Concrete bounded RAM and disk benchmark implementation.
#[derive(Clone, Copy, Debug, Default)]
pub struct LocalHostBenchmarks;

impl HostBenchmarkIo for LocalHostBenchmarks {
	fn benchmark_ram(&self, domain: &RamDomain, plan: BoundedBenchmarkPlan) -> ProbeResult<RamMeasurement> {
		require_bounded(plan)?;
		let bytes = usize::try_from(plan.buffer_bytes.get()).map_err(|_| ProbeError::Benchmark("RAM benchmark buffer does not fit usize".to_owned()))?;
		let source = vec![0xa5u8; bytes];
		let mut destination = vec![0u8; bytes];
		let start = Instant::now();
		let mut iterations = 0u32;
		while iterations < plan.iterations && start.elapsed() < plan.maximum_duration {
			destination.copy_from_slice(&source);
			black_box(&destination);
			iterations += 1;
		}
		let rate = observed_rate(plan.buffer_bytes, iterations, start.elapsed())?;
		Ok(RamMeasurement {
			capacity: measured(domain.capacity_hint),
			transfer_rate: measured(rate),
		})
	}

	fn benchmark_storage(&self, domain: &StorageDomain, plan: BoundedBenchmarkPlan) -> ProbeResult<StorageMeasurement> {
		require_bounded(plan)?;
		let bytes = usize::try_from(plan.buffer_bytes.get()).map_err(|_| ProbeError::Benchmark("disk benchmark buffer does not fit usize".to_owned()))?;
		let mut temporary = TemporaryProbeFile::create(&domain.benchmark_root)?;
		let buffer = vec![0x5au8; bytes];

		let write_start = Instant::now();
		let mut writes = 0u32;
		while writes < plan.iterations && write_start.elapsed() < plan.maximum_duration {
			temporary
				.file
				.seek(SeekFrom::Start(0))
				.map_err(|error| ProbeError::io("seek", &temporary.path, error))?;
			temporary
				.file
				.write_all(&buffer)
				.map_err(|error| ProbeError::io("write", &temporary.path, error))?;
			temporary
				.file
				.sync_data()
				.map_err(|error| ProbeError::io("sync", &temporary.path, error))?;
			writes += 1;
		}
		let write_rate = observed_rate(plan.buffer_bytes, writes, write_start.elapsed())?;

		let mut read_buffer = vec![0u8; bytes];
		let read_start = Instant::now();
		let mut reads = 0u32;
		while reads < plan.iterations && read_start.elapsed() < plan.maximum_duration {
			temporary
				.file
				.seek(SeekFrom::Start(0))
				.map_err(|error| ProbeError::io("seek", &temporary.path, error))?;
			temporary
				.file
				.read_exact(&mut read_buffer)
				.map_err(|error| ProbeError::io("read", &temporary.path, error))?;
			black_box(&read_buffer);
			reads += 1;
		}
		let read_rate = observed_rate(plan.buffer_bytes, reads, read_start.elapsed())?;
		let capacity = filesystem_available_bytes(&domain.benchmark_root)?;
		Ok(StorageMeasurement {
			capacity: measured(capacity),
			read_rate: measured(read_rate),
			write_rate: measured(write_rate),
		})
	}
}

#[derive(Debug)]
struct TemporaryProbeFile {
	path: PathBuf,
	file: File,
}

impl TemporaryProbeFile {
	fn create(directory: &Path) -> ProbeResult<Self> {
		static NEXT: AtomicU64 = AtomicU64::new(1);
		for _ in 0..64 {
			let nonce = NEXT.fetch_add(1, Ordering::Relaxed);
			let path = directory.join(format!(".recipe-probe-{}-{nonce}.tmp", std::process::id()));
			match OpenOptions::new()
				.read(true)
				.write(true)
				.create_new(true)
				.open(&path)
			{
				Ok(file) => return Ok(Self { path, file }),
				Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
				Err(error) => return Err(ProbeError::io("create", path, error)),
			}
		}
		Err(ProbeError::Benchmark(
			"could not allocate a unique temporary disk-probe file".to_owned(),
		))
	}
}

impl Drop for TemporaryProbeFile {
	fn drop(&mut self) { let _ = fs::remove_file(&self.path); }
}

fn filesystem_available_bytes(path: &Path) -> ProbeResult<ByteCount> {
	let statistics = rustix::fs::statvfs(path).map_err(|error| ProbeError::io("statvfs", path, error))?;
	let value = statistics
		.f_bavail
		.checked_mul(statistics.f_frsize)
		.ok_or_else(|| {
			ProbeError::Benchmark(format!(
				"available-byte count overflowed for {}",
				path.display()
			))
		})?;
	Ok(ByteCount::new(value))
}

fn observed_rate(bytes: ByteCount, iterations: u32, elapsed: std::time::Duration) -> ProbeResult<BytesPerSecond> {
	if iterations == 0 || elapsed.is_zero() {
		return Err(ProbeError::Benchmark(
			"bounded benchmark completed no measurable work".to_owned(),
		));
	}
	let total = u128::from(bytes.get())
		.checked_mul(u128::from(iterations))
		.and_then(|value| value.checked_mul(1_000_000_000))
		.ok_or_else(|| ProbeError::Benchmark("rate accounting overflow".to_owned()))?;
	let rate = total / elapsed.as_nanos();
	let rate = u64::try_from(rate).map_err(|_| ProbeError::Benchmark("measured rate exceeds u64".to_owned()))?;
	BytesPerSecond::new(rate).map_err(|error| ProbeError::Benchmark(format!("invalid measured rate: {error}")))
}

fn require_bounded(plan: BoundedBenchmarkPlan) -> ProbeResult<()> {
	if plan.is_bounded() {
		Ok(())
	} else {
		Err(ProbeError::Benchmark(
			"benchmark plan must have nonzero byte, iteration, and duration bounds".to_owned(),
		))
	}
}

fn measured<T>(value: T) -> Property<T> { Property::new(value, PropertyProvenance::Measured) }

fn parse_memtotal(input: &str) -> ProbeResult<u64> {
	for line in input.lines() {
		if let Some(index) = line.find("MemTotal:") {
			let rest = &line[index + "MemTotal:".len()..];
			let kib = rest
				.split_whitespace()
				.find_map(|part| part.parse::<u64>().ok())
				.ok_or_else(|| ProbeError::Discovery(format!("invalid MemTotal line {line:?}")))?;
			return kib
				.checked_mul(1024)
				.ok_or_else(|| ProbeError::Discovery("MemTotal byte conversion overflow".to_owned()));
		}
	}
	Err(ProbeError::Discovery(
		"memory information contains no MemTotal".to_owned(),
	))
}

fn read_trimmed(path: impl AsRef<Path>) -> ProbeResult<String> {
	let path = path.as_ref();
	let value = fs::read_to_string(path).map_err(|error| ProbeError::io("read", path, error))?;
	let value = value.trim();
	if value.is_empty() {
		Err(ProbeError::Discovery(format!(
			"{} is empty",
			path.display()
		)))
	} else {
		Ok(value.to_owned())
	}
}

fn read_first_available(paths: &[PathBuf]) -> ProbeResult<String> {
	for path in paths {
		if let Ok(value) = read_trimmed(path) {
			return Ok(value);
		}
	}
	Err(ProbeError::Discovery(
		"no stable machine identity is available".to_owned(),
	))
}

fn join_available(paths: &[PathBuf], separator: &str) -> Option<String> {
	let values = paths
		.iter()
		.filter_map(|path| read_trimmed(path).ok())
		.collect::<Vec<_>>();
	(!values.is_empty()).then(|| values.join(separator))
}

fn label(value: impl Into<String>) -> ProbeResult<Label> { Label::new(value).map_err(|error| ProbeError::Discovery(error.to_string())) }

fn unescape_mount(value: &str) -> String {
	value.replace("\\040", " ")
		.replace("\\011", "\t")
		.replace("\\012", "\n")
		.replace("\\134", "\\")
}
