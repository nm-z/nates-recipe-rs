use std::fmt;

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

/// Production GPU half of `ProbeEngine`.
///
/// Each call revalidates exact backend-library and hardware identities.
/// A missing CUDA or ROCr library is allowed only when PCI discovery finds no
/// accelerator for that vendor. Hardware without its native runtime, or an
/// existing backend that fails to load or enumerate, is not silently treated
/// as absent.
pub struct NativeGpuProbe {
	backends: Vec<Box<dyn Backend>>,
	exhaustive: bool,
}

impl NativeGpuProbe {
	pub fn new(config: NativeProbeConfig) -> ProbeResult<Self> {
		validate_config(&config)?;
		let backends: Vec<Box<dyn Backend>> = vec![
			Box::new(CudaBackend::new(&config)?),
			Box::new(HsaBackend::new(&config)?),
		];
		Ok(Self {
			backends,
			exhaustive: true,
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
			backends: vec![Box::new(CudaBackend::new(&config)?)],
			exhaustive: false,
		})
	}

	/// Construct an HSA-only, deliberately non-exhaustive hardware diagnostic.
	///
	/// See [`Self::cuda_diagnostic`]; normal profile construction must use
	/// [`Self::new`].
	pub fn hsa_diagnostic(config: NativeProbeConfig) -> ProbeResult<Self> {
		validate_config(&config)?;
		Ok(Self {
			backends: vec![Box::new(HsaBackend::new(&config)?)],
			exhaustive: false,
		})
	}

	#[cfg(test)]
	pub(crate) fn from_backends(backends: Vec<Box<dyn Backend>>) -> Self {
		Self {
			backends,
			exhaustive: true,
		}
	}
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
			devices.extend(backend.discover()?);
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

	use recipe_core::{
		ByteCount, BytesPerSecond, Digest, FlopsPerSecond, Label, Property, PropertyProvenance, TargetIdentity,
		ToolchainIdentity, TransferLaneCount, TransportKind,
	};
	use recipe_probe::{GpuInventory, LinkDuplex};

	use super::*;

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
			maximum_concurrent_tasks: 1,
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
}
