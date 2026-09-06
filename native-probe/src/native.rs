use std::{
	fmt, fs,
	path::{Path, PathBuf},
};

use recipe_probe::{BoundedBenchmarkPlan, GpuBenchmarkIo, GpuDescriptor, GpuDiscovery, GpuInventory, GpuMeasurement, ProbeError, ProbeResult};

use crate::{config::NativeProbeConfig, cuda::CudaBackend, hsa::HsaBackend};

pub(crate) trait Backend: fmt::Debug {
	fn discover(&self) -> ProbeResult<Vec<GpuDescriptor>>;
	fn benchmark(&self, device: &GpuDescriptor, plan: BoundedBenchmarkPlan) -> ProbeResult<GpuMeasurement>;
}

#[derive(Debug)]
enum NativeBackend {
	Cuda(CudaBackend),
	Hsa(HsaBackend),
}

impl NativeBackend {
	fn backend(&self) -> &dyn Backend {
		match self {
			Self::Cuda(backend) => backend,
			Self::Hsa(backend) => backend,
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

	pub(crate) fn cuda_backend(&self) -> Option<&CudaBackend> {
		self.backends.iter().find_map(|backend| {
			match backend {
				NativeBackend::Cuda(backend) => Some(backend),
				NativeBackend::Hsa(_) => None,
			}
		})
	}

	pub(crate) fn hsa_backend(&self) -> Option<&HsaBackend> {
		self.backends.iter().find_map(|backend| {
			match backend {
				NativeBackend::Hsa(backend) => Some(backend),
				NativeBackend::Cuda(_) => None,
			}
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
	bytes.len() == 12 && bytes[0..4].iter().all(u8::is_ascii_hexdigit) && bytes[4] == b':' && bytes[5..7].iter().all(u8::is_ascii_hexdigit) && bytes[7] == b':' && bytes[8..10].iter().all(u8::is_ascii_hexdigit) && bytes[10] == b'.' && matches!(bytes[11], b'0'..=b'7')
}

fn enabled_display_connectors(pci_sysfs_root: &Path, bdf: &str) -> ProbeResult<u32> {
	let device_root = pci_sysfs_root.join(bdf);
	let metadata = fs::metadata(&device_root).map_err(|error| ProbeError::io("inspect GPU PCI device", &device_root, error))?;
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
		let connectors = fs::read_dir(&path).map_err(|error| ProbeError::io("enumerate GPU DRM connectors", &path, error))?;
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
			let state = fs::read_to_string(&enabled_path).map_err(|error| ProbeError::io("read GPU DRM connector state", &enabled_path, error))?;
			match state.trim() {
				"enabled" => {
					enabled = enabled
						.checked_add(1)
						.ok_or_else(|| ProbeError::Discovery("enabled GPU display connector count overflowed u32".to_owned()))?;
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
