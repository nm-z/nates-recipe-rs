use std::env;
use std::ffi::OsString;
use std::fmt::Write as _;
use std::fs::{self, DirBuilder};
use std::os::unix::fs::{DirBuilderExt as _, MetadataExt as _, PermissionsExt as _};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use recipe_core::{Digest, Label};
use recipe_kernel::{OfflineToolchain, PinnedTool};
use recipe_native_probe::{
	BackendLibrary, CudaProbeConfig, HsaProbeConfig, KernelBuildConfig, NativeGpuProbe, NativeProbeConfig,
};
use recipe_probe::local::{LocalHostBenchmarks, LocalSystemDiscovery};
use recipe_probe::{ExplicitPathProfileCache, HostDiscovery as _, ProbeEngine, SeedContract};

const USAGE: &str = "\
Recipe command line

Usage:
  recipe probe [OPTIONS]

The zero-argument probe uses the embedded theoretical seed contract, discovers
the current bare-metal machine, benchmarks every discovered device and link,
and writes an identity-keyed measured profile under the user's private cache.

Options:
  --contract PATH       Use another theoretical probe seed contract
  --profile PATH        Write/load this absolute measured-profile path
  --cuda-driver PATH    Exact CUDA Driver library candidate (repeatable)
  --hsa-runtime PATH    Exact ROCr/HSA runtime candidate (repeatable)
  --llvm-opt PATH       Exact LLVM IR verifier
  --llvm-llc PATH       Exact LLVM code generator
  --lld PATH            Exact ELF linker
  --ptxas PATH          Exact NVIDIA PTX assembler
  -h, --help             Show this help
";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct ProbeOptions {
	contract: Option<PathBuf>,
	profile: Option<PathBuf>,
	cuda_libraries: Vec<PathBuf>,
	hsa_libraries: Vec<PathBuf>,
	llvm_opt: Option<PathBuf>,
	llvm_llc: Option<PathBuf>,
	lld: Option<PathBuf>,
	ptxas: Option<PathBuf>,
}

pub fn main() -> ExitCode {
	match run(env::args_os().skip(1).collect()) {
		Ok(()) => ExitCode::SUCCESS,
		Err(error) => {
			eprintln!("recipe: {error}");
			ExitCode::FAILURE
		}
	}
}

fn run(arguments: Vec<OsString>) -> Result<(), String> {
	let Some(command) = arguments.first() else {
		return Err(format!("missing command\n\n{USAGE}"));
	};
	if command == "-h" || command == "--help" {
		print!("{USAGE}");
		return Ok(());
	}
	if command != "probe" {
		return Err(format!(
			"unknown command {:?}; expected `probe`\n\n{USAGE}",
			command
		));
	}
	if arguments.len() == 2 && (arguments[1] == "-h" || arguments[1] == "--help") {
		print!("{USAGE}");
		return Ok(());
	}
	let options = parse_probe_options(&arguments[1..])?;
	run_probe(options)
}

fn parse_probe_options(arguments: &[OsString]) -> Result<ProbeOptions, String> {
	let mut options = ProbeOptions::default();
	let mut index = 0;
	while index < arguments.len() {
		let option = arguments[index]
			.to_str()
			.ok_or_else(|| "command-line options must be valid UTF-8".to_owned())?;
		let value = arguments
			.get(index + 1)
			.ok_or_else(|| format!("option {option} requires a path argument"))?;
		let path = PathBuf::from(value);
		match option {
			"--contract" => set_once(&mut options.contract, path, option)?,
			"--profile" => set_once(&mut options.profile, path, option)?,
			"--cuda-driver" => options.cuda_libraries.push(path),
			"--hsa-runtime" => options.hsa_libraries.push(path),
			"--llvm-opt" => set_once(&mut options.llvm_opt, path, option)?,
			"--llvm-llc" => set_once(&mut options.llvm_llc, path, option)?,
			"--lld" => set_once(&mut options.lld, path, option)?,
			"--ptxas" => set_once(&mut options.ptxas, path, option)?,
			_ => return Err(format!("unknown probe option {option:?}\n\n{USAGE}")),
		}
		index += 2;
	}
	Ok(options)
}

fn set_once(slot: &mut Option<PathBuf>, value: PathBuf, option: &str) -> Result<(), String> {
	if slot.replace(value).is_some() {
		Err(format!("{option} may be supplied only once"))
	} else {
		Ok(())
	}
}

fn run_probe(options: ProbeOptions) -> Result<(), String> {
	require_bare_metal()?;
	let seed = match &options.contract {
		Some(path) => SeedContract::read(path).map_err(|error| error.to_string())?,
		None => {
			SeedContract::parse(include_str!("../topology/contract.toml")).map_err(|error| error.to_string())?
		}
	};

	let state_root = private_state_root()?;
	let scratch = state_root.join("scratch");
	ensure_private_directory(&scratch)?;

	let host = LocalSystemDiscovery::with_benchmark_roots(vec![state_root.clone()]);
	let inventory = host.discover_host().map_err(|error| error.to_string())?;
	let host_memory_key = inventory
		.ram
		.first()
		.map(|domain| domain.key.clone())
		.ok_or_else(|| "host discovery returned no RAM domain".to_owned())?;
	let native = NativeGpuProbe::new(native_config(&options, host_memory_key, scratch)?)
		.map_err(|error| error.to_string())?;
	let host_benchmarks = LocalHostBenchmarks;
	let engine = ProbeEngine::new(&host, &native, &host_benchmarks, &native);
	let peers = [];
	let identity = engine
		.current_cache_identity(&seed, &peers)
		.map_err(|error| error.to_string())?;

	let profile_path = match options.profile {
		Some(path) => path,
		None => {
			let directory = state_root.join("profiles");
			ensure_private_directory(&directory)?;
			directory.join(format!(
				"measured-v{}-{}.recipe-profile",
				identity.schema,
				hex(identity.digest)
			))
		}
	};
	let cache = ExplicitPathProfileCache::new(&profile_path).map_err(|error| error.to_string())?;
	let was_cached = profile_path.exists();
	let profile = engine
		.load_or_probe_and_store(&seed, &peers, &cache)
		.map_err(|error| error.to_string())?;

	println!("profile={}", profile_path.display());
	println!(
		"source={}",
		if was_cached {
			"validated-cache"
		} else {
			"fresh-measurement"
		}
	);
	println!("cache_identity={}", hex(profile.cache_identity.digest));
	println!(
		"topology_identity={}",
		hex(profile.topology.identity.digest())
	);
	println!(
		"discovery_identity={}",
		hex(profile.discovery.identity.digest())
	);
	println!("machines={}", profile.topology.machines.len());
	println!("devices={}", profile.topology.devices.len());
	println!("directed_links={}", profile.topology.links.len());
	Ok(())
}

fn native_config(
	options: &ProbeOptions,
	host_memory_key: Label,
	scratch_parent: PathBuf,
) -> Result<NativeProbeConfig, String> {
	let verifier = required_tool(
		options.llvm_opt.as_deref(),
		&[
			"/usr/bin/opt",
			"/usr/local/bin/opt",
			"/usr/lib/llvm-22/bin/opt",
			"/usr/lib/llvm-21/bin/opt",
			"/usr/lib/llvm-20/bin/opt",
			"/usr/lib/llvm-19/bin/opt",
			"/opt/llvm/bin/opt",
		],
		"LLVM opt",
	)?;
	let llvm_codegen = required_tool(
		options.llvm_llc.as_deref(),
		&[
			"/usr/bin/llc",
			"/usr/local/bin/llc",
			"/usr/lib/llvm-22/bin/llc",
			"/usr/lib/llvm-21/bin/llc",
			"/usr/lib/llvm-20/bin/llc",
			"/usr/lib/llvm-19/bin/llc",
			"/opt/llvm/bin/llc",
		],
		"LLVM llc",
	)?;
	let elf_linker = optional_tool(
		options.lld.as_deref(),
		&[
			"/usr/bin/ld.lld",
			"/usr/local/bin/ld.lld",
			"/usr/lib/llvm-22/bin/ld.lld",
			"/usr/lib/llvm-21/bin/ld.lld",
			"/usr/lib/llvm-20/bin/ld.lld",
			"/opt/llvm/bin/ld.lld",
		],
		"LLVM lld",
	)?;
	let ptx_assembler = optional_tool(
		options.ptxas.as_deref(),
		&[
			"/opt/cuda-11.8/bin/ptxas",
			"/opt/cuda-11.7/bin/ptxas",
			"/opt/cuda-11.6/bin/ptxas",
			"/opt/cuda-11.5/bin/ptxas",
			"/opt/cuda-11.4/bin/ptxas",
			"/usr/local/cuda-11.8/bin/ptxas",
			"/usr/local/cuda-11.4/bin/ptxas",
			"/opt/cuda/bin/ptxas",
			"/usr/local/cuda/bin/ptxas",
			"/usr/bin/ptxas",
		],
		"NVIDIA ptxas",
	)?;

	let cuda_candidates = configured_or_default(
		&options.cuda_libraries,
		&[
			"/usr/lib/x86_64-linux-gnu/libcuda.so.1",
			"/usr/lib64/libcuda.so.1",
			"/usr/lib/libcuda.so.1",
			"/usr/local/nvidia/lib64/libcuda.so.1",
		],
	);
	let hsa_candidates = configured_or_default(
		&options.hsa_libraries,
		&[
			"/opt/rocm/lib/libhsa-runtime64.so.1",
			"/usr/lib/x86_64-linux-gnu/libhsa-runtime64.so.1",
			"/usr/lib64/libhsa-runtime64.so.1",
			"/usr/lib/libhsa-runtime64.so.1",
		],
	);
	Ok(NativeProbeConfig {
		host_memory_key,
		pci_sysfs_root: PathBuf::from("/sys/bus/pci/devices"),
		cuda: CudaProbeConfig {
			library: BackendLibrary {
				candidates: cuda_candidates,
			},
			// PTX 7.4 is accepted by the R470 deployment fixture and remains
			// valid input to newer pinned assemblers.
			ptx_isa: 74,
		},
		hsa: HsaProbeConfig {
			library: BackendLibrary {
				candidates: hsa_candidates,
			},
			code_object_version: 6,
		},
		kernels: KernelBuildConfig {
			toolchain: OfflineToolchain {
				verifier,
				llvm_codegen,
				elf_linker,
				ptx_assembler,
			},
			release: Label::new("auto-pinned-local-tools-and-benchmark-v3").map_err(|error| error.to_string())?,
			scratch_parent,
			// The structurally inspected HSACO path is live-validated through
			// 64 dependent FMAs on gfx1101. This is long enough to amortize
			// launch overhead while keeping the bounded probe responsive.
			fma_chain_length: 64,
		},
	})
}

fn configured_or_default(configured: &[PathBuf], defaults: &[&str]) -> Vec<PathBuf> {
	if configured.is_empty() {
		defaults.iter().map(PathBuf::from).collect()
	} else {
		configured.to_vec()
	}
}

fn required_tool(explicit: Option<&Path>, candidates: &[&str], name: &str) -> Result<PinnedTool, String> {
	optional_tool(explicit, candidates, name)?
		.ok_or_else(|| format!("{name} was not found in the fixed candidate set; supply its exact absolute path"))
}

fn optional_tool(explicit: Option<&Path>, candidates: &[&str], name: &str) -> Result<Option<PinnedTool>, String> {
	if let Some(path) = explicit {
		return PinnedTool::inspect(path)
			.map(Some)
			.map_err(|error| format!("{name}: {error}"));
	}
	for candidate in candidates {
		let path = Path::new(candidate);
		match fs::symlink_metadata(path) {
			Ok(_) => {
				return PinnedTool::inspect(path)
					.map(Some)
					.map_err(|error| format!("{name}: {error}"));
			}
			Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
			Err(error) => {
				return Err(format!(
					"inspect {name} candidate {}: {error}",
					path.display()
				));
			}
		}
	}
	Ok(None)
}

fn private_state_root() -> Result<PathBuf, String> {
	let base = match env::var_os("XDG_CACHE_HOME") {
		Some(value) if Path::new(&value).is_absolute() => PathBuf::from(value),
		Some(_) | None => {
			let home = env::var_os("HOME")
				.ok_or_else(|| "neither an absolute XDG_CACHE_HOME nor HOME is available".to_owned())?;
			let home = fs::canonicalize(&home).map_err(|error| format!("canonicalize user home: {error}"))?;
			home.join(".cache")
		}
	};
	if !base.exists() {
		DirBuilder::new()
			.recursive(true)
			.mode(0o700)
			.create(&base)
			.map_err(|error| format!("create cache base {}: {error}", base.display()))?;
	}
	let base =
		fs::canonicalize(&base).map_err(|error| format!("canonicalize cache base {}: {error}", base.display()))?;
	let root = base.join("recipe-next");
	ensure_private_directory(&root)?;
	Ok(root)
}

fn ensure_private_directory(path: &Path) -> Result<(), String> {
	if !path.is_absolute() {
		return Err(format!(
			"private directory {} is not absolute",
			path.display()
		));
	}
	if !path.exists() {
		DirBuilder::new()
			.mode(0o700)
			.create(path)
			.map_err(|error| format!("create private directory {}: {error}", path.display()))?;
	}
	let metadata = fs::symlink_metadata(path)
		.map_err(|error| format!("inspect private directory {}: {error}", path.display()))?;
	if metadata.file_type().is_symlink() || !metadata.is_dir() {
		return Err(format!(
			"private path {} must be a real directory",
			path.display()
		));
	}
	if metadata.permissions().mode() & 0o077 != 0 {
		return Err(format!(
			"private directory {} grants group or other permissions",
			path.display()
		));
	}
	let effective_uid = fs::metadata("/proc/self")
		.map_err(|error| format!("inspect /proc/self ownership: {error}"))?
		.uid();
	if metadata.uid() != effective_uid {
		return Err(format!(
			"private directory {} is not owned by the effective user",
			path.display()
		));
	}
	let canonical = fs::canonicalize(path)
		.map_err(|error| format!("canonicalize private directory {}: {error}", path.display()))?;
	if canonical != path {
		return Err(format!(
			"private directory {} is not canonical or contains a symlink",
			path.display()
		));
	}
	Ok(())
}

fn require_bare_metal() -> Result<(), String> {
	for marker in ["/.dockerenv", "/run/.containerenv"] {
		if Path::new(marker).exists() {
			return Err(format!(
				"`recipe probe` requires bare metal; container marker {marker} exists"
			));
		}
	}
	if let Ok(cgroup) = fs::read_to_string("/proc/1/cgroup") {
		let lower = cgroup.to_ascii_lowercase();
		for marker in ["docker", "containerd", "kubepods", "libpod", "lxc"] {
			if lower.contains(marker) {
				return Err(format!(
					"`recipe probe` requires bare metal; PID 1 cgroup reports {marker}"
				));
			}
		}
	}
	if let Ok(status) = fs::read_to_string("/proc/self/status")
		&& let Some(line) = status.lines().find(|line| line.starts_with("NSpid:"))
		&& line.split_ascii_whitespace().skip(1).count() > 1
	{
		return Err("`recipe probe` requires bare metal; the process is inside a PID namespace".to_owned());
	}
	Ok(())
}

fn hex(digest: Digest) -> String {
	let mut output = String::with_capacity(64);
	for byte in digest.bytes() {
		write!(output, "{byte:02x}").expect("writing to String");
	}
	output
}
