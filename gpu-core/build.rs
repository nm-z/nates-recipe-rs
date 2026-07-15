#![allow(
	unsafe_code,
	reason = "HIP_PLATFORM env toggle for the NVIDIA hipcc pass"
)]
use core::array::from_fn;
use std::env;
use std::fs;
use std::io;
use std::os::unix::fs as unix_fs;
use std::path::{Path, PathBuf};
use std::process;

/// Count of HIP memory-API choke points tracked by the ledger scan.
const NAPI: usize = 8;

/// GPU BLAS libraries banned from direct use — `hipBLAS` is the only sanctioned entry.
const BANNED: [&str; 2] = ["rocblas", "cublas"];

/// HIP memory-API choke points; each may appear at most twice (decl + single ledgered call site).
const APIS: [&str; NAPI] = [
	"hipMemcpyAsync(",
	"hipHostMalloc(",
	"hipMallocAsync(",
	"hipMemsetAsync(",
	"hipHostFree(",
	"hipFreeAsync(",
	"hipMemPoolSetAttribute(",
	"hipMemPoolTrimTo(",
];

/// Static hipcc flags for the AMD kernel archive; dynamic paths ride the `dynamics` slice.
const AMD_FLAGS: [&str; 4] = ["-x", "hip", "-fPIC", "-O3"];

/// Static nvcc flags for the NVIDIA device-library (rocPRIM/hipCUB) kernel archive.
const NV_DEVLIB_FLAGS: [&str; 9] = [
	"-x",
	"cu",
	"-O3",
	"-diag-suppress",
	"2810",
	"-D__HIP_PLATFORM_NVIDIA__=1",
	"-DTHRUST_IGNORE_CUB_VERSION_CHECK",
	"-Xcompiler",
	"-fPIC",
];

/// Static hipcc flags for the NVIDIA plain-kernel archive compiled under `HIP_PLATFORM=nvidia`.
const NV_PLAIN_FLAGS: [&str; 4] = ["-fPIC", "-O3", "-diag-suppress", "2810"];

/// Static nvcc flags for the NVIDIA hip→cu memory shim.
const NV_SHIM_FLAGS: [&str; 3] = ["-O3", "-Xcompiler", "-fPIC"];

/// The HIP backend platform, decided solely by `hipconfig --platform`.
#[derive(Clone, Copy)]
enum Platform {
	/// AMD `ROCm`: hipBLAS/hipSOLVER/hipFFT forward to rocBLAS/rocSOLVER/rocFFT.
	Amd,
	/// NVIDIA CUDA: hipBLAS/hipSOLVER/hipFFT wrap cuBLAS/cuSOLVER/cuFFT.
	Nvidia,
}

/// Prints one `cargo:` build directive to stdout with a trailing newline.
/// Write errors are dropped, never propagated.
fn put(s: &str) {
	use std::io::Write as _;
	let mut o = io::stdout();
	drop(o.write_all(s.as_bytes()));
	drop(o.write_all(b"\n"));
}

/// Emits a `cargo:rustc-link-search=native=<path>` directive.
fn link_search(path: &str) {
	put(&format!("cargo:rustc-link-search=native={path}"));
}

/// Emits a `cargo:rustc-link-lib=dylib=<name>` directive.
fn link_lib(name: &str) {
	put(&format!("cargo:rustc-link-lib=dylib={name}"));
}

/// Unwraps `v`, or errors the build with `what` as the message.
fn need<T>(v: Option<T>, what: &str) -> Result<T, String> {
	return v.ok_or_else(|| return what.to_owned());
}

/// Returns `key`'s environment value, or `default` when unset.
fn env_or(key: &str, default: String) -> String {
	return env::var(key).unwrap_or(default);
}

/// Prefixes `env_or(key, default)` with `prefix`, yielding a compiler flag string.
fn envflag(prefix: &str, key: &str, default: String) -> String {
	return format!("{prefix}{}", env_or(key, default));
}

/// Runs `hipconfig <flag>`, returning its trimmed stdout.
/// Errs (naming the hip-runtime package to install) if the binary is missing or the call fails.
fn hipconfig(flag: &str) -> Result<String, String> {
	let out = match process::Command::new("hipconfig").arg(flag).output() {
		Ok(out) => out,
		Err(e) if e.kind() == io::ErrorKind::NotFound => {
			return Err(
				"hipconfig not found; install hip-runtime-amd or hip-runtime-nvidia"
					.to_owned(),
			);
		}
		Err(e) => return Err(format!("hipconfig {flag}: cannot run: {e}")),
	};
	if !out.status.success() {
		return Err(format!(
			"hipconfig {flag}: {}",
			String::from_utf8_lossy(&out.stderr)
		));
	}
	return Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned());
}

/// Recursively scans every `.rs` under `dir`: errors on a banned BLAS symbol, else records each HIP
/// memory-API choke-point occurrence into `sites` keyed by API index. Recurses into subdirectories.
fn scan_sources(dir: &Path, sites: &mut [Vec<String>]) -> Result<(), String> {
	let Ok(rd) = fs::read_dir(dir) else {
		return Ok(());
	};
	for entry in rd.flatten() {
		let p = entry.path();
		if p.is_dir() {
			scan_sources(&p, sites)?;
			continue;
		}
		if !p.extension().is_some_and(|x| return x == "rs") {
			continue;
		}
		let text = fs::read_to_string(&p).unwrap_or_default();
		for (i, line) in text.lines().enumerate() {
			let low = line.to_lowercase();
			for pat in &BANNED {
				if low.contains(pat) {
					return Err(format!(
						"{}:{}: direct {pat} banned — call hipBLAS (hipblas*) instead",
						p.display(),
						i.saturating_add(1)
					));
				}
			}
			for (k, api) in APIS.iter().enumerate() {
				if line.contains(api) {
					sites[k].push(format!("{}:{}", p.display(), i.saturating_add(1)));
				}
			}
		}
	}
	return Ok(());
}

/// Errors if any API choke point in `sites` exceeds two occurrences (decl + single call site);
/// walks the parallel `sites`/`apis` tails by recursion.
fn check_counts(sites: &[Vec<String>], apis: &[&str]) -> Result<(), String> {
	let Some((s, srest)) = sites.split_first() else {
		return Ok(());
	};
	let Some((api, arest)) = apis.split_first() else {
		return Ok(());
	};
	if s.len() > 2 {
		return Err(format!(
			"{api}: {} occurrences (max 2 = decl + choke call site): {}",
			s.len(),
			s.join(", ")
		));
	}
	return check_counts(srest, arest);
}

/// Recursively appends every `.hip` file under `dir` to `out` in filesystem order.
/// An unreadable directory is silently skipped, not an error.
fn collect_hip_files(dir: &Path, out: &mut Vec<PathBuf>) {
	let Ok(entries) = fs::read_dir(dir) else {
		return;
	};
	for entry in entries.flatten() {
		let path = entry.path();
		if path.is_dir() {
			collect_hip_files(&path, out);
			continue;
		}
		if path.extension().is_some_and(|x| return x == "hip") {
			out.push(path);
		}
	}
}

/// Copies each `.hip` in `files` to a flat `.cu` under `cudir`, bucketing device-library sources
/// (rocPRIM/hipCUB) into `dev` and the rest into `plain`; recurses down the slice tail.
fn stage_cus(
	files: &[PathBuf],
	cudir: &str,
	dev: &mut Vec<String>,
	plain: &mut Vec<String>,
) -> Result<(), String> {
	let Some((src_path, rest)) = files.split_first() else {
		return Ok(());
	};
	let src = need(src_path.to_str(), "non-utf8 kernel path")?;
	let rel = src_path
		.strip_prefix("src/kernels")
		.map_err(|e| return format!("{src}: outside src/kernels: {e}"))?;
	let stem = need(rel.to_str(), "non-utf8 kernel path")?.replace(['/', '\\', '.'], "_");
	let cu = format!("{cudir}/{stem}.cu");
	fs::copy(src, &cu).map_err(|e| return format!("copy {src} to {cu}: {e}"))?;
	let text = fs::read_to_string(src_path).unwrap_or_default();
	if text.contains("rocprim") || text.contains("hipcub") {
		dev.push(cu);
	} else {
		plain.push(cu);
	}
	return stage_cus(rest, cudir, dev, plain);
}

/// Recursively removes every stale `.o` and `libhip*.a` under `dir` so the full kernel set recompiles fresh.
/// Unreadable dirs and failed removals are silently skipped; never fails the build.
fn sweep(dir: &Path) {
	let Ok(rd) = fs::read_dir(dir) else {
		return;
	};
	for entry in rd.flatten() {
		let p = entry.path();
		if p.is_dir() {
			sweep(&p);
		} else {
			let name = p.file_name().and_then(|n| return n.to_str()).unwrap_or("");
			let is_a = Path::new(name)
				.extension()
				.is_some_and(|x| return x.eq_ignore_ascii_case("a"));
			let stale = p.extension().is_some_and(|x| return x == "o")
				|| (name.starts_with("libhip") && is_a);
			if stale {
				drop(fs::remove_file(&p));
			}
		}
	}
}

/// Recreates `path` empty, erroring on a failed create.
fn fresh_dir(path: &str) -> Result<(), String> {
	drop(fs::remove_dir_all(path));
	return fs::create_dir_all(path).map_err(|e| return format!("mkdir {path}: {e}"));
}

/// Emits the three hipBLAS/hipSOLVER/hipFFT link directives shared by both platforms.
fn link_hip() {
	link_lib("hipblas");
	link_lib("hipsolver");
	link_lib("hipfft");
}

/// Compiles `files` with `compiler` into the static archive `name`, applying `statics` then
/// `dynamics` flags followed by one `-isystem <dir>` pair per `includes` entry.
fn compile_kernels<P: AsRef<Path>>(
	compiler: &str,
	statics: &[&str],
	dynamics: &[&str],
	includes: &[&str],
	files: &[P],
	name: &str,
) {
	let mut b = cc::Build::new();
	b.compiler(compiler).no_default_flags(true).warnings(false);
	for f in statics.iter().chain(dynamics.iter()) {
		b.flag(f);
	}
	for inc in includes {
		b.flag("-isystem").flag(inc);
	}
	b.files(files).compile(name);
}

/// Compiles the kernel set when non-empty, optionally toggling the nvidia platform env around the call.
fn compile_if(
	compiler: &str,
	statics: &[&str],
	dynamics: &[&str],
	includes: &[&str],
	files: &[String],
	name: &str,
	nv_env: bool,
) {
	if files.is_empty() {
		return;
	}
	if nv_env {
		// SAFETY: the build script is single-threaded here; nothing else reads HIP_PLATFORM.
		unsafe {
			env::set_var("HIP_PLATFORM", "nvidia");
		}
	}
	compile_kernels(compiler, statics, dynamics, includes, files, name);
	if nv_env {
		// SAFETY: the build script is single-threaded here; nothing else reads HIP_PLATFORM.
		unsafe {
			env::remove_var("HIP_PLATFORM");
		}
	}
}

fn main() -> Result<(), String> {
	let mut sites: [Vec<String>; NAPI] = from_fn(|_u| return Vec::new());
	scan_sources(Path::new("src"), &mut sites)?;
	check_counts(&sites, &APIS)?;
	let platform = match hipconfig("--platform")?.as_str() {
		"amd" => Platform::Amd,
		"nvidia" => Platform::Nvidia,
		other => {
			return Err(format!(
				"hipconfig --platform returned {other:?}; install hip-runtime-amd or hip-runtime-nvidia"
			));
		}
	};
	let out_dir = need(env::var("OUT_DIR").ok(), "OUT_DIR unset")?;
	let mut hip_files = Vec::new();
	collect_hip_files(Path::new("src/kernels"), &mut hip_files);
	hip_files.sort();
	put("cargo:rerun-if-changed=src/kernels");
	sweep(Path::new(&out_dir));
	let rocm = hipconfig("--rocmpath")?;
	if rocm.is_empty() {
		return Err("hipconfig --rocmpath: empty output".to_owned());
	}
	link_search(&format!("{rocm}/lib"));
	match platform {
		Platform::Amd => {
			let hipcc = env::var("HIPCC").unwrap_or_else(|_e| {
				let p = format!("{rocm}/bin/hipcc");
				if Path::new(&p).exists() {
					return p;
				}
				return format!("{rocm}/bin/amdclang++");
			});
			let rp = format!("--rocm-path={rocm}");
			let ip = envflag("-I", "ROCM_EXTRA_INCLUDE", format!("{rocm}/include"));
			let ap = envflag("--offload-arch=", "GPU_ARCH", "gfx1101".to_owned());
			let dflags = [rp.as_str(), ip.as_str(), ap.as_str()];
			compile_kernels(&hipcc, &AMD_FLAGS, &dflags, &[], &hip_files, "hipkernels");
			link_lib("amdhip64");
			link_search(&env_or("ROCM_EXTRA_LIB", format!("{rocm}/lib")));
			link_hip();
		}
		Platform::Nvidia => {
			let cuda = env_or("CUDA_PATH", "/opt/cuda".to_owned());
			let nvcc = env_or("NVCC", format!("{cuda}/bin/nvcc"));
			let arch = format!("-arch={}", env_or("CUDA_ARCH", "sm_86".to_owned()));
			let compat = format!(
				"{}/src/nvidia_compat",
				need(
					env::var("CARGO_MANIFEST_DIR").ok(),
					"CARGO_MANIFEST_DIR unset"
				)?
			);
			let shfl = format!("{compat}/hip_shfl_compat.cuh");
			let nvhip = format!("{out_dir}/nvhip");
			fresh_dir(&nvhip)?;
			drop(unix_fs::symlink(
				format!("{rocm}/include/hip"),
				format!("{nvhip}/hip"),
			));
			let cudir = format!("{out_dir}/cu");
			fresh_dir(&cudir)?;
			let (mut device_lib_cus, mut plain_cus): (Vec<String>, Vec<String>) =
				(Vec::new(), Vec::new());
			stage_cus(&hip_files, &cudir, &mut device_lib_cus, &mut plain_cus)?;
			compile_if(
				&nvcc,
				&NV_DEVLIB_FLAGS,
				&[arch.as_str(), "-include", shfl.as_str()],
				&[compat.as_str(), nvhip.as_str()],
				&device_lib_cus,
				"hipkernels_devlib",
				false,
			);
			compile_if(
				&env_or("HIPCC", format!("{rocm}/bin/hipcc")),
				&NV_PLAIN_FLAGS,
				&[arch.as_str(), "-include", shfl.as_str()],
				&[],
				&plain_cus,
				"hipkernels",
				true,
			);
			put("cargo:rerun-if-changed=src/shim_nvidia.cu");
			compile_kernels(
				&nvcc,
				&NV_SHIM_FLAGS,
				&[arch.as_str(), format!("-I{cuda}/include").as_str()],
				&[],
				&["src/shim_nvidia.cu"],
				"hipshim",
			);
			link_hip();
			link_search(&format!("{cuda}/lib64"));
			for lib in ["cudart", "cublas", "cusolver", "cufft"] {
				link_lib(lib);
			}
		}
	}
	link_lib("stdc++");
	return Ok(());
}
