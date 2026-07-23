use std::collections::{BTreeMap, BTreeSet};

use recipe_core::{DeviceId, MachineId, RunId};
use recipe_cuda::{Context, ContextFlags, DeploymentIdentity};
use recipe_host::{DiskFileSpec, HostBackendConfig, HostDeviceBinding};
use recipe_hsa::{DiscoveredAgent, Session};
use recipe_native_executor::{CudaBinding, HsaBinding};
use recipe_probe::{
	GpuDiscovery as _, HostInventory, MeasuredProfile, ProbeError, ProbeResult, ResolvedGpuDevice,
	ResolvedLocalInventory,
};

use crate::NativeGpuProbe;
use crate::config::NativeProbeConfig;
use crate::cuda::CudaBackend;
use crate::hsa::{HsaBackend, exact_target};

const CUDA_BACKEND: &str = "nvidia-cuda-driver";
const HSA_BACKEND: &str = "amd-rocr-hsa";

/// Exact, preparation-scoped native bindings for the current measured
/// machine.
///
/// Bindings borrow their realized CUDA contexts and HSA sessions, so they
/// cannot outlive this function's callback or leak a runtime handle into a
/// later dynamic placement path.
#[derive(Clone, Debug)]
pub struct NativeExecutionBindings<'cuda, 'hsa> {
	machine: MachineId,
	cuda: Vec<CudaBinding<'cuda>>,
	hsa: Vec<HsaBinding<'hsa>>,
}

impl<'cuda, 'hsa> NativeExecutionBindings<'cuda, 'hsa> {
	#[must_use]
	pub const fn machine(&self) -> MachineId {
		self.machine
	}

	#[must_use]
	pub fn cuda(&self) -> &[CudaBinding<'cuda>] {
		&self.cuda
	}

	#[must_use]
	pub fn hsa(&self) -> &[HsaBinding<'hsa>] {
		&self.hsa
	}

	#[must_use]
	pub fn into_parts(self) -> (Vec<CudaBinding<'cuda>>, Vec<HsaBinding<'hsa>>) {
		(self.cuda, self.hsa)
	}
}

#[derive(Clone, Debug)]
struct ExpectedGpu {
	device: DeviceId,
	key: String,
}

#[derive(Debug)]
struct RealizedCuda {
	device: DeviceId,
	context: Context,
	deployment: DeploymentIdentity,
}

struct RealizedHsa<'runtime> {
	device: DeviceId,
	session: Session<'runtime>,
	target_id: String,
	queue_packets: u32,
}

/// Construct the host half of a local backend from exact resolved RAM/storage
/// origins.
///
/// Disk paths are deterministic, run-scoped children of each automatically
/// discovered storage root. Construction performs no I/O; the host backend
/// later uses `create_new` while realizing the candidate, so an overlapping
/// run ID fails without overwriting an existing file.
pub fn host_backend_config_from_inventory(
	inventory: &ResolvedLocalInventory<'_>,
	run: RunId,
	worker_threads: usize,
	staging_bytes_per_slot: usize,
) -> recipe_host::Result<HostBackendConfig> {
	let mut bindings = inventory
		.ram()
		.iter()
		.map(|resolved| HostDeviceBinding::Ram {
			device: resolved.device(),
		})
		.collect::<Vec<_>>();
	for resolved in inventory.storage() {
		let stem = format!(".recipe-run-{run}-device-{}", resolved.device());
		let root = &resolved.domain().benchmark_root;
		bindings.push(HostDeviceBinding::Disk {
			device: resolved.device(),
			arena: DiskFileSpec::new(root.join(format!("{stem}-arena")))?,
			reservation: DiskFileSpec::new(root.join(format!("{stem}-reservation")))?,
		});
	}
	HostBackendConfig::new(worker_threads, staging_bytes_per_slot, bindings)
}

/// Reopen every current GPU by the exact keys retained by `recipe probe`, then
/// lend its native bindings to one preparation/execution scope.
///
/// The current host fingerprint and complete RAM/storage/GPU key sets must
/// still equal the measured profile. There is deliberately no ordinal,
/// product-name, capacity, or performance fallback.
pub fn with_native_execution_bindings<T>(
	config: &NativeProbeConfig,
	profile: &MeasuredProfile,
	host: &HostInventory,
	operation: impl for<'cuda, 'hsa> FnOnce(NativeExecutionBindings<'cuda, 'hsa>) -> ProbeResult<T>,
) -> ProbeResult<T> {
	let inventory = {
		let probe = NativeGpuProbe::new(config.clone())?;
		probe.discover_all()?
	};
	let resolved = profile.resolve_local_inventory(host, &inventory)?;
	let machine = resolved.machine();
	let (expected_cuda, expected_hsa) = partition_expected(resolved.gpu())?;

	let cuda_backend = CudaBackend::new(config)?;
	let realized_cuda = realize_cuda(&cuda_backend, &expected_cuda)?;
	let cuda_bindings = realized_cuda
		.iter()
		.map(|realized| {
			CudaBinding::new(
				realized.device,
				&realized.context,
				realized.deployment.clone(),
			)
		})
		.collect::<Vec<_>>();

	let hsa_backend = HsaBackend::new(config)?;
	let mut operation = Some(operation);
	let invoked = hsa_backend.with_runtime(|library, runtime| {
		let discovery = runtime
			.discover()
			.map_err(|error| binding_error(format!("exhaustive HSA reopening: {error}")))?;
		let system = discovery.system().clone();
		let realized_hsa = realize_hsa(
			&hsa_backend,
			library,
			&system,
			discovery.into_agents(),
			&expected_hsa,
		)?;
		let hsa_bindings = realized_hsa
			.iter()
			.map(|realized| {
				HsaBinding::new(
					realized.device,
					&realized.session,
					realized.target_id.clone(),
					config.hsa.code_object_version,
					realized.queue_packets,
				)
			})
			.collect::<Vec<_>>();
		let operation = operation
			.take()
			.expect("native binding operation is invoked at most once");
		operation(NativeExecutionBindings {
			machine,
			cuda: cuda_bindings.clone(),
			hsa: hsa_bindings,
		})
	})?;
	match invoked {
		Some(result) => Ok(result),
		None if expected_hsa.is_empty() => {
			let operation = operation
				.take()
				.expect("native binding operation was not invoked");
			operation(NativeExecutionBindings {
				machine,
				cuda: cuda_bindings,
				hsa: Vec::new(),
			})
		}
		None => Err(binding_error(
			"measured HSA GPU origins exist but no AMD runtime surface was reopened",
		)),
	}
}

fn partition_expected(
	resolved: &[ResolvedGpuDevice<'_>],
) -> ProbeResult<(BTreeMap<String, ExpectedGpu>, BTreeMap<String, ExpectedGpu>)> {
	let mut cuda = BTreeMap::new();
	let mut hsa = BTreeMap::new();
	for resolved in resolved {
		let descriptor = resolved.descriptor();
		let expected = ExpectedGpu {
			device: resolved.device(),
			key: descriptor.key.as_str().to_owned(),
		};
		let destination = match descriptor.target.backend.as_str() {
			CUDA_BACKEND => &mut cuda,
			HSA_BACKEND => &mut hsa,
			backend => {
				return Err(binding_error(format!(
					"GPU origin {} uses unsupported native backend {backend}",
					descriptor.key
				)));
			}
		};
		if destination.insert(expected.key.clone(), expected).is_some() {
			return Err(binding_error(
				"resolved GPU origins contain a duplicate key",
			));
		}
	}
	Ok((cuda, hsa))
}

fn realize_cuda(backend: &CudaBackend, expected: &BTreeMap<String, ExpectedGpu>) -> ProbeResult<Vec<RealizedCuda>> {
	let Some((library, driver, discovery)) = backend.open()? else {
		return if expected.is_empty() {
			Ok(Vec::new())
		} else {
			Err(binding_error(
				"measured CUDA GPU origins exist but no NVIDIA Driver surface was reopened",
			))
		};
	};
	let mut seen = BTreeSet::new();
	let mut realized = Vec::new();
	for device in &discovery.devices {
		let descriptor = backend.descriptor(&library, &discovery, device)?;
		let Some(expected) = expected.get(descriptor.key.as_str()) else {
			return Err(binding_error(format!(
				"reopened CUDA device {} was not present in the measured profile",
				descriptor.key
			)));
		};
		if !seen.insert(expected.key.clone()) {
			return Err(binding_error(format!(
				"CUDA Driver reopened duplicate device {}",
				expected.key
			)));
		}
		let deployment = DeploymentIdentity::from_discovery(&discovery, device).ok_or_else(|| {
			binding_error(format!(
				"CUDA deployment identity for {} is not part of its discovery snapshot",
				expected.key
			))
		})?;
		let context = Context::create(&driver, device, ContextFlags::default()).map_err(|error| {
			binding_error(format!(
				"create CUDA context for measured device {}: {error}",
				expected.key
			))
		})?;
		realized.push(RealizedCuda {
			device: expected.device,
			context,
			deployment,
		});
	}
	require_all_reopened("CUDA", expected, &seen)?;
	realized.sort_by_key(|binding| binding.device);
	Ok(realized)
}

fn realize_hsa<'runtime>(
	backend: &HsaBackend,
	library: &crate::identity::PinnedLibrary,
	system: &recipe_hsa::SystemDescription,
	agents: Vec<DiscoveredAgent<'runtime>>,
	expected: &BTreeMap<String, ExpectedGpu>,
) -> ProbeResult<Vec<RealizedHsa<'runtime>>> {
	let mut seen = BTreeSet::new();
	let mut realized = Vec::new();
	for agent in agents {
		let Some(descriptor) = backend.descriptor(library, system, agent.description())? else {
			continue;
		};
		let Some(expected) = expected.get(descriptor.key.as_str()) else {
			return Err(binding_error(format!(
				"reopened HSA device {} was not present in the measured profile",
				descriptor.key
			)));
		};
		if !seen.insert(expected.key.clone()) {
			return Err(binding_error(format!(
				"ROCr reopened duplicate device {}",
				expected.key
			)));
		}
		let target_id = exact_target(agent.description())?.as_str().to_owned();
		let queue_packets = agent
			.description()
			.queue
			.ok_or_else(|| {
				binding_error(format!(
					"measured HSA device {} no longer advertises a queue",
					expected.key
				))
			})?
			.minimum_packets;
		let session = agent.into_session().map_err(|error| {
			binding_error(format!(
				"create HSA session for measured device {}: {error}",
				expected.key
			))
		})?;
		realized.push(RealizedHsa {
			device: expected.device,
			session,
			target_id,
			queue_packets,
		});
	}
	require_all_reopened("HSA", expected, &seen)?;
	realized.sort_by_key(|binding| binding.device);
	Ok(realized)
}

fn require_all_reopened(
	backend: &str,
	expected: &BTreeMap<String, ExpectedGpu>,
	seen: &BTreeSet<String>,
) -> ProbeResult<()> {
	let missing = expected
		.keys()
		.filter(|key| !seen.contains(*key))
		.cloned()
		.collect::<Vec<_>>();
	if missing.is_empty() {
		Ok(())
	} else {
		Err(binding_error(format!(
			"{backend} did not reopen measured GPU origins {missing:?}"
		)))
	}
}

fn binding_error(detail: impl Into<String>) -> ProbeError {
	ProbeError::Discovery(detail.into())
}
