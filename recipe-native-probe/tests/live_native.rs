use std::env;
use std::path::PathBuf;
use std::time::Duration;

use recipe_core::{ByteCount, Label, RunId};
use recipe_kernel::{OfflineToolchain, PinnedTool};
use recipe_native_probe::{
	BackendLibrary, CudaProbeConfig, HsaProbeConfig, KernelBuildConfig, NativeGpuProbe, NativeProbeConfig,
	host_backend_config_from_inventory, with_native_execution_bindings,
};
use recipe_probe::local::{LocalHostBenchmarks, LocalSystemDiscovery};
use recipe_probe::{
	BoundedBenchmarkPlan, GpuBenchmarkIo as _, GpuDiscovery as _, HostDiscovery as _, ProbeEngine, SeedContract,
};

#[test]
#[ignore = "requires explicitly configured bare-metal GPU libraries and compiler tools"]
fn discovers_and_benchmarks_every_live_gpu() -> Result<(), Box<dyn std::error::Error>> {
	let config = configured_probe()?;
	let diagnostic_backend = env::var("RECIPE_PROBE_DIAGNOSTIC_BACKEND").ok();
	let probe = match diagnostic_backend.as_deref() {
		None => NativeGpuProbe::new(config)?,
		Some("cuda") => NativeGpuProbe::cuda_diagnostic(config)?,
		Some("hsa") => NativeGpuProbe::hsa_diagnostic(config)?,
		Some(other) => {
			return Err(
				std::io::Error::other(format!("unknown RECIPE_PROBE_DIAGNOSTIC_BACKEND {other:?}")).into(),
			);
		}
	};
	let inventory = probe.discover_all()?;
	assert_eq!(inventory.exhaustive, diagnostic_backend.is_none());
	assert!(!inventory.devices.is_empty());
	let plan = BoundedBenchmarkPlan {
		buffer_bytes: ByteCount::new(4 * 1024 * 1024),
		iterations: 4,
		maximum_duration: Duration::from_secs(2),
	};
	for device in &inventory.devices {
		let measurement = probe.benchmark_gpu(device, plan)?;
		assert_eq!(measurement.capacity.value, device.capacity_hint);
		assert!(measurement.calculation_rate.value.get() > 0);
		assert!(measurement.memory_rate.value.get() > 0);
		assert!(measurement.host_to_device_rate.value.get() > 0);
		assert!(measurement.device_to_host_rate.value.get() > 0);
	}
	Ok(())
}

#[test]
#[ignore = "requires explicitly configured bare-metal GPU libraries and compiler tools"]
fn measured_origins_reopen_exact_native_bindings() -> Result<(), Box<dyn std::error::Error>> {
	if env::var_os("RECIPE_PROBE_DIAGNOSTIC_BACKEND").is_some() {
		return Err(std::io::Error::other("exact binding test requires exhaustive dual-backend mode").into());
	}
	let mut config = configured_probe()?;
	let benchmark_root = config.kernels.scratch_parent.clone();
	let host = LocalSystemDiscovery::with_benchmark_roots(vec![benchmark_root]);
	let inventory = host.discover_host()?;
	config.host_memory_key = inventory
		.ram
		.first()
		.ok_or_else(|| std::io::Error::other("live host has no RAM domain"))?
		.key
		.clone();
	let native = NativeGpuProbe::new(config.clone())?;
	let host_benchmarks = LocalHostBenchmarks;
	let engine = ProbeEngine::new(&host, &native, &host_benchmarks, &native);
	let seed = SeedContract::parse(include_str!("../../topology/contract.toml"))?;
	let profile = engine.probe(&seed, &[])?;
	let live_gpus = native.discover_all()?;
	let resolved = profile.resolve_local_inventory(&inventory, &live_gpus)?;
	let host_config = host_backend_config_from_inventory(&resolved, RunId::new(91), 1, 4096)?;
	assert_eq!(
		host_config.bindings().len(),
		resolved.ram().len() + resolved.storage().len()
	);
	let expected_cuda = profile
		.origins
		.gpu
		.iter()
		.filter(|origin| origin.key.as_str().starts_with("cuda:"))
		.count();
	let expected_hsa = profile
		.origins
		.gpu
		.iter()
		.filter(|origin| origin.key.as_str().starts_with("hsa:"))
		.count();
	with_native_execution_bindings(&config, &profile, &inventory, |bindings| {
		assert_eq!(bindings.cuda().len(), expected_cuda);
		assert_eq!(bindings.hsa().len(), expected_hsa);
		assert_eq!(bindings.machine(), profile.topology.machines[0].id);
		Ok(())
	})?;
	Ok(())
}

fn configured_probe() -> Result<NativeProbeConfig, Box<dyn std::error::Error>> {
	let verifier = PinnedTool::inspect(required_path("RECIPE_PROBE_OPT")?)?;
	let llvm_codegen = PinnedTool::inspect(required_path("RECIPE_PROBE_LLC")?)?;
	let elf_linker = optional_path("RECIPE_PROBE_LLD")
		.map(PinnedTool::inspect)
		.transpose()?;
	let ptx_assembler = optional_path("RECIPE_PROBE_PTXAS")
		.map(PinnedTool::inspect)
		.transpose()?;
	Ok(NativeProbeConfig {
		host_memory_key: Label::new("ram:local")?,
		pci_sysfs_root: PathBuf::from("/sys/bus/pci/devices"),
		cuda: CudaProbeConfig {
			library: BackendLibrary {
				candidates: explicit_candidates("RECIPE_PROBE_CUDA_LIBRARY")?,
			},
			ptx_isa: required_u16("RECIPE_PROBE_PTX_ISA")?,
		},
		hsa: HsaProbeConfig {
			library: BackendLibrary {
				candidates: explicit_candidates("RECIPE_PROBE_HSA_LIBRARY")?,
			},
			code_object_version: required_u8("RECIPE_PROBE_HSA_CODE_OBJECT_VERSION")?,
		},
		kernels: KernelBuildConfig {
			toolchain: OfflineToolchain {
				verifier,
				llvm_codegen,
				elf_linker,
				ptx_assembler,
			},
			release: Label::new(env::var("RECIPE_PROBE_TOOLCHAIN_RELEASE")?)?,
			scratch_parent: required_path("RECIPE_PROBE_SCRATCH_PARENT")?,
			fma_chain_length: env::var("RECIPE_PROBE_FMA_CHAIN").map_or(Ok(64), |value| value.parse())?,
		},
	})
}

fn required_path(name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
	Ok(PathBuf::from(env::var_os(name).ok_or_else(|| {
		std::io::Error::other(format!("{name} is required"))
	})?))
}

fn optional_path(name: &str) -> Option<PathBuf> {
	env::var_os(name).map(PathBuf::from)
}

fn explicit_candidates(name: &str) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
	Ok(match env::var_os(name) {
		Some(path) => vec![PathBuf::from(path)],
		None => Vec::new(),
	})
}

fn required_u16(name: &str) -> Result<u16, Box<dyn std::error::Error>> {
	Ok(env::var(name)?.parse()?)
}

fn required_u8(name: &str) -> Result<u8, Box<dyn std::error::Error>> {
	Ok(env::var(name)?.parse()?)
}
