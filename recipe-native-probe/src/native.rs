use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use recipe_probe::{
	BoundedBenchmarkPlan, GpuBenchmarkIo, GpuDescriptor, GpuDiscovery, GpuInventory, GpuMeasurement, ProbeError,
	ProbeResult,
};

use crate::config::NativeProbeConfig;
use crate::cuda::CudaBackend;
use crate::hsa::HsaBackend;

pub(crate) trait Backend: fmt::Debug {
	fn discover(&self) -> ProbeResult<Vec<GpuDescriptor>>;
	fn benchmark(&self, device: &GpuDescriptor, plan: BoundedBenchmarkPlan) -> ProbeResult<GpuMeasurement>;
}

#[derive(Debug)]
enum NativeBackend {
	Cuda(CudaBackend),
	Hsa(HsaBackend),
	#[cfg(test)]
	Test(Box<dyn Backend>),
}

impl NativeBackend {
	fn backend(&self) -> &dyn Backend {
		match self {
			Self::Cuda(backend) => backend,
			Self::Hsa(backend) => backend,
			#[cfg(test)]
			Self::Test(backend) => backend.as_ref(),
		}
	}
}

/// Production GPU half of `ProbeEngine`.
///
/// Each call revalidates exact backend-library and hardware identities.
/// A missing CUDA or ROCr library is allowed only when PCI discovery finds no
/// accelerator for that vendor. Hardware without its native runtime, or an
/// existing backend that fails to load or enumerate, is not silently treated
/// as absent.
pub struct NativeGpuProbe {
	backends: Vec<NativeBackend>,
	exhaustive: bool,
	pci_sysfs_root: PathBuf,
}

impl NativeGpuProbe {
	pub fn new(config: NativeProbeConfig) -> ProbeResult<Self> {
		validate_config(&config)?;
		let backends = vec![
			NativeBackend::Cuda(CudaBackend::new(&config)?),
			NativeBackend::Hsa(HsaBackend::new(&config)?),
		];
		Ok(Self {
			backends,
			exhaustive: true,
			pci_sysfs_root: config.pci_sysfs_root,
		})
	}

	/// Construct a CUDA-only, deliberately non-exhaustive hardware diagnostic.
	///
	/// This cannot produce an accepted measured profile because
	/// [`GpuInventory::exhaustive`] is false. It exists for validating one
	/// backend on mixed machines where another required device is temporarily
	/// unavailable.
	pub fn cuda_diagnostic(config: NativeProbeConfig) -> ProbeResult<Self> {
		validate_config(&config)?;
		Ok(Self {
			backends: vec![NativeBackend::Cuda(CudaBackend::new(&config)?)],
			exhaustive: false,
			pci_sysfs_root: config.pci_sysfs_root,
		})
	}

	/// Construct an HSA-only, deliberately non-exhaustive hardware diagnostic.
	///
	/// See [`Self::cuda_diagnostic`]; normal profile construction must use
	/// [`Self::new`].
	pub fn hsa_diagnostic(config: NativeProbeConfig) -> ProbeResult<Self> {
		validate_config(&config)?;
		Ok(Self {
			backends: vec![NativeBackend::Hsa(HsaBackend::new(&config)?)],
			exhaustive: false,
			pci_sysfs_root: config.pci_sysfs_root,
		})
	}

	#[cfg(test)]
	pub(crate) fn from_backends(backends: Vec<Box<dyn Backend>>) -> Self {
		Self {
			backends: backends.into_iter().map(NativeBackend::Test).collect(),
			exhaustive: true,
			pci_sysfs_root: PathBuf::from("/sys/bus/pci/devices"),
		}
	}

	pub(crate) fn cuda_backend(&self) -> Option<&CudaBackend> {
		self.backends.iter().find_map(|backend| match backend {
			NativeBackend::Cuda(backend) => Some(backend),
			NativeBackend::Hsa(_) => None,
			#[cfg(test)]
			NativeBackend::Test(_) => None,
		})
	}

	pub(crate) fn hsa_backend(&self) -> Option<&HsaBackend> {
		self.backends.iter().find_map(|backend| match backend {
			NativeBackend::Hsa(backend) => Some(backend),
			NativeBackend::Cuda(_) => None,
			#[cfg(test)]
			NativeBackend::Test(_) => None,
		})
	}

	pub(crate) fn enabled_display_connectors(&self, origin: &str) -> ProbeResult<u32> {
		let bdf = origin
			.rsplit_once('@')
			.map(|(_, bdf)| bdf)
			.filter(|bdf| valid_pci_bdf(bdf))
			.ok_or_else(|| {
				ProbeError::Discovery(format!(
					"GPU origin {origin:?} has no canonical PCI BDF suffix"
				))
			})?;
		enabled_display_connectors(&self.pci_sysfs_root, bdf)
	}
}

fn valid_pci_bdf(value: &str) -> bool {
	let bytes = value.as_bytes();
	bytes.len() == 12
		&& bytes[0..4].iter().all(u8::is_ascii_hexdigit)
		&& bytes[4] == b':'
		&& bytes[5..7].iter().all(u8::is_ascii_hexdigit)
		&& bytes[7] == b':'
		&& bytes[8..10].iter().all(u8::is_ascii_hexdigit)
		&& bytes[10] == b'.'
		&& matches!(bytes[11], b'0'..=b'7')
}

fn enabled_display_connectors(pci_sysfs_root: &Path, bdf: &str) -> ProbeResult<u32> {
	let device_root = pci_sysfs_root.join(bdf);
	let metadata = fs::metadata(&device_root)
		.map_err(|error| ProbeError::io("inspect GPU PCI device", &device_root, error))?;
	if !metadata.is_dir() {
		return Err(ProbeError::Discovery(format!(
			"GPU PCI root {} is not a directory",
			device_root.display()
		)));
	}
	let drm_root = device_root.join("drm");
	let cards = match fs::read_dir(&drm_root) {
		Ok(entries) => entries,
		Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
		Err(error) => {
			return Err(ProbeError::io(
				"enumerate GPU DRM devices",
				&drm_root,
				error,
			));
		}
	};
	let mut card_paths = Vec::new();
	for entry in cards {
		let entry = entry.map_err(|error| ProbeError::io("enumerate GPU DRM device", &drm_root, error))?;
		let name = entry.file_name();
		let Some(name) = name.to_str() else {
			return Err(ProbeError::Discovery(format!(
				"GPU DRM entry under {} is not valid UTF-8",
				drm_root.display()
			)));
		};
		let Some(index) = name.strip_prefix("card") else {
			continue;
		};
		if index.is_empty() || !index.bytes().all(|byte| byte.is_ascii_digit()) {
			continue;
		}
		let path = entry.path();
		if path.is_dir() {
			card_paths.push((name.to_owned(), path));
		}
	}
	card_paths.sort_by(|left, right| left.0.cmp(&right.0));
	let mut enabled = 0_u32;
	for (card, path) in card_paths {
		let connectors =
			fs::read_dir(&path).map_err(|error| ProbeError::io("enumerate GPU DRM connectors", &path, error))?;
		let prefix = format!("{card}-");
		let mut connector_paths = Vec::new();
		for entry in connectors {
			let entry = entry.map_err(|error| ProbeError::io("enumerate GPU DRM connector", &path, error))?;
			let name = entry.file_name();
			let Some(name) = name.to_str() else {
				return Err(ProbeError::Discovery(format!(
					"GPU DRM connector under {} is not valid UTF-8",
					path.display()
				)));
			};
			if name.starts_with(&prefix) && entry.path().is_dir() {
				connector_paths.push(entry.path());
			}
		}
		connector_paths.sort();
		for connector in connector_paths {
			let enabled_path = connector.join("enabled");
			let state = fs::read_to_string(&enabled_path)
				.map_err(|error| ProbeError::io("read GPU DRM connector state", &enabled_path, error))?;
			match state.trim() {
				"enabled" => {
					enabled = enabled.checked_add(1).ok_or_else(|| {
						ProbeError::Discovery(
							"enabled GPU display connector count overflowed u32".to_owned(),
						)
					})?;
				}
				"disabled" => {}
				state => {
					return Err(ProbeError::Discovery(format!(
						"GPU DRM connector {} reported invalid enabled state {state:?}",
						connector.display()
					)));
				}
			}
		}
	}
	Ok(enabled)
}

fn validate_config(config: &NativeProbeConfig) -> ProbeResult<()> {
	if config.kernels.fma_chain_length == 0 {
		return Err(ProbeError::Discovery(
			"native FLOP benchmark requires a nonzero FMA chain".to_owned(),
		));
	}
	if !config.kernels.scratch_parent.is_absolute() {
		return Err(ProbeError::Discovery(format!(
			"kernel scratch parent {} is not absolute",
			config.kernels.scratch_parent.display()
		)));
	}
	Ok(())
}

impl fmt::Debug for NativeGpuProbe {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("NativeGpuProbe")
			.field("backend_count", &self.backends.len())
			.finish_non_exhaustive()
	}
}

impl GpuDiscovery for NativeGpuProbe {
	fn discover_all(&self) -> ProbeResult<GpuInventory> {
		let mut devices = Vec::new();
		for backend in &self.backends {
			devices.extend(backend.backend().discover()?);
		}
		devices.sort_by(|left, right| left.key.cmp(&right.key));
		for pair in devices.windows(2) {
			if pair[0].key == pair[1].key {
				return Err(ProbeError::Discovery(format!(
					"native backends emitted duplicate GPU key {}",
					pair[0].key
				)));
			}
		}
		Ok(GpuInventory {
			exhaustive: self.exhaustive,
			devices,
		})
	}
}

impl GpuBenchmarkIo for NativeGpuProbe {
	fn benchmark_gpu(&self, device: &GpuDescriptor, plan: BoundedBenchmarkPlan) -> ProbeResult<GpuMeasurement> {
		if !plan.is_bounded() {
			return Err(ProbeError::Benchmark(
				"native GPU benchmark received an unbounded plan".to_owned(),
			));
		}
		let mut owner = None;
		for backend in &self.backends {
			let backend = backend.backend();
			let exact = backend
				.discover()?
				.into_iter()
				.any(|candidate| candidate == *device);
			if exact {
				if owner.is_some() {
					return Err(ProbeError::Benchmark(format!(
						"multiple native backends claim exact GPU {}",
						device.key
					)));
				}
				owner = Some(backend);
			}
		}
		owner.ok_or_else(|| {
			ProbeError::Benchmark(format!(
				"GPU {} identity changed after discovery",
				device.key
			))
		})?
		.benchmark(device, plan)
	}
}

#[cfg(test)]
mod tests {
	use std::collections::VecDeque;
	use std::sync::Mutex;
	use std::sync::atomic::{AtomicU64, Ordering};

	use recipe_core::{
		ByteCount, BytesPerSecond, Digest, FlopsPerSecond, Label, Property, PropertyProvenance, TargetIdentity,
		ToolchainIdentity, TransferLaneCount, TransportKind,
	};
	use recipe_probe::{GpuInventory, LinkDuplex};

	use super::*;

	static DISPLAY_NONCE: AtomicU64 = AtomicU64::new(0);

	#[derive(Debug)]
	struct FakeBackend {
		discoveries: Mutex<VecDeque<Vec<GpuDescriptor>>>,
		measurement: GpuMeasurement,
	}

	impl Backend for FakeBackend {
		fn discover(&self) -> ProbeResult<Vec<GpuDescriptor>> {
			let mut discoveries = self.discoveries.lock().expect("fake lock");
			match discoveries.len() {
				0 => Ok(Vec::new()),
				1 => Ok(discoveries.front().expect("one discovery").clone()),
				_ => Ok(discoveries.pop_front().expect("queued discovery")),
			}
		}

		fn benchmark(&self, _device: &GpuDescriptor, _plan: BoundedBenchmarkPlan) -> ProbeResult<GpuMeasurement> {
			Ok(self.measurement)
		}
	}

	fn label(value: &str) -> Label {
		Label::new(value).expect("test label")
	}

	fn descriptor(key: &str) -> GpuDescriptor {
		GpuDescriptor {
			key: label(key),
			name: label("Fake GPU"),
			host_memory_key: label("ram0"),
			target: TargetIdentity {
				backend: label("fake"),
				architecture: label("fake1"),
				abi: label("fake-abi"),
			},
			capacity_hint: ByteCount::new(1024),
			driver: label("driver"),
			runtime_abi: label("runtime"),
			firmware: label("firmware"),
			link_identity: label("link"),
			transport_kind: TransportKind::Pcie,
			toolchain: ToolchainIdentity {
				name: label("tool"),
				version: label("1"),
				digest: Digest::new([1; 32]),
			},
			duplex: LinkDuplex::Full,
			host_to_device_maximum_inflight: TransferLaneCount::new(1).expect("test lanes"),
			device_to_host_maximum_inflight: TransferLaneCount::new(1).expect("test lanes"),
			asynchronous_submission: true,
			maximum_submission_queues: 1,
			maximum_concurrent_tasks: 1,
			subgroup_lanes: 32,
			maximum_workgroup_lanes: 256,
			maximum_shared_memory_per_workgroup: ByteCount::new(64 * 1024),
			transfer_overlaps_calculation: true,
		}
	}

	fn measurement() -> GpuMeasurement {
		GpuMeasurement {
			capacity: Property::new(ByteCount::new(1024), PropertyProvenance::Measured),
			calculation_rate: Property::new(
				FlopsPerSecond::new(10).expect("rate"),
				PropertyProvenance::Measured,
			),
			memory_rate: Property::new(
				BytesPerSecond::new(20).expect("rate"),
				PropertyProvenance::Measured,
			),
			host_to_device_rate: Property::new(
				BytesPerSecond::new(30).expect("rate"),
				PropertyProvenance::Measured,
			),
			device_to_host_rate: Property::new(
				BytesPerSecond::new(40).expect("rate"),
				PropertyProvenance::Measured,
			),
		}
	}

	#[test]
	fn combines_all_backend_inventories_deterministically() {
		let left = FakeBackend {
			discoveries: Mutex::new(VecDeque::from([vec![descriptor("z")]])),
			measurement: measurement(),
		};
		let right = FakeBackend {
			discoveries: Mutex::new(VecDeque::from([vec![descriptor("a")]])),
			measurement: measurement(),
		};
		let probe = NativeGpuProbe::from_backends(vec![Box::new(left), Box::new(right)]);
		let GpuInventory {
			exhaustive,
			devices,
		} = probe.discover_all().expect("discover fake GPUs");
		assert!(exhaustive);
		assert_eq!(
			devices
				.iter()
				.map(|device| device.key.as_str())
				.collect::<Vec<_>>(),
			["a", "z"]
		);
	}

	#[test]
	fn identity_change_between_discovery_and_benchmark_fails_closed() {
		let original = descriptor("gpu");
		let mut changed = original.clone();
		changed.runtime_abi = label("changed-runtime");
		let backend = FakeBackend {
			discoveries: Mutex::new(VecDeque::from([vec![changed]])),
			measurement: measurement(),
		};
		let probe = NativeGpuProbe::from_backends(vec![Box::new(backend)]);
		let plan = BoundedBenchmarkPlan {
			buffer_bytes: ByteCount::new(4096),
			iterations: 1,
			maximum_duration: std::time::Duration::from_millis(1),
		};
		let error = probe
			.benchmark_gpu(&original, plan)
			.expect_err("identity drift must not benchmark a different GPU");
		assert!(error.to_string().contains("identity changed"));
	}

	#[test]
	fn display_probe_counts_only_enabled_connectors_on_the_exact_bdf() {
		let root = std::env::temp_dir().join(format!(
			"recipe-display-probe-{}-{}",
			std::process::id(),
			DISPLAY_NONCE.fetch_add(1, Ordering::Relaxed)
		));
		let card = root.join("0000:03:00.0").join("drm").join("card1");
		let enabled = card.join("card1-DP-1");
		let disabled = card.join("card1-HDMI-A-1");
		fs::create_dir_all(&enabled).expect("create enabled connector");
		fs::create_dir_all(&disabled).expect("create disabled connector");
		fs::write(enabled.join("enabled"), b"enabled\n").expect("write enabled connector");
		fs::write(disabled.join("enabled"), b"disabled\n").expect("write disabled connector");

		assert_eq!(
			enabled_display_connectors(&root, "0000:03:00.0").expect("probe display connectors"),
			1
		);
		fs::remove_dir_all(root).expect("remove display fixture");
	}

	#[test]
	fn display_probe_treats_an_absent_drm_surface_as_headless() {
		let root = std::env::temp_dir().join(format!(
			"recipe-headless-probe-{}-{}",
			std::process::id(),
			DISPLAY_NONCE.fetch_add(1, Ordering::Relaxed)
		));
		fs::create_dir_all(root.join("0000:03:00.0")).expect("create headless PCI device");
		assert_eq!(
			enabled_display_connectors(&root, "0000:03:00.0").expect("probe headless GPU"),
			0
		);
		fs::remove_dir_all(root).expect("remove headless fixture");
	}

	#[test]
	fn display_probe_rejects_malformed_gpu_origins() {
		let mut probe = NativeGpuProbe::from_backends(Vec::new());
		probe.pci_sysfs_root = PathBuf::from("/does/not/matter");
		let error = probe
			.enabled_display_connectors("hsa:gpu@../../device")
			.expect_err("malformed BDF must fail closed");
		assert!(error.to_string().contains("canonical PCI BDF"));
	}
}
