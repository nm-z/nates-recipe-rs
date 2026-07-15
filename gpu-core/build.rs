#![allow(
	unsafe_code,
	reason = "HIP_PLATFORM env toggle for the NVIDIA hipcc pass"
)]
use std::env;
use std::fs;
use std::io;
use std::os::unix::fs as unix_fs;
use std::path::{Path, PathBuf};
use std::process;

// Backend truth comes from hipconfig alone — runtime detection, not hardware:
// no env overrides, no filesystem probes, no hardcoded default. Installing is
// the package manager's job, never the build's: a missing or undecided HIP
// runtime fails the build with the package names.
#[derive(Clone, Copy)]
enum Platform {
	Amd,
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

/// Prints a line to stderr and exits the build with status 1; never returns.
/// The sole fatal-error path for every build-script check (chokepoint scan, hipconfig, platform detect).
fn die(s: &str) -> ! {
	use std::io::Write as _;
	let mut e = io::stderr();
	drop(e.write_all(s.as_bytes()));
	drop(e.write_all(b"\n"));
	process::exit(1)
}

/// Unwraps `v`, or aborts the build via `die` (stderr + exit 1) with `what` as the message.
fn need<T>(v: Option<T>, what: &str) -> T {
	match v {
		Some(t) => return t,
		None => die(what),
	}
}

/// Runs `hipconfig <flag>`, returning its trimmed stdout.
/// Exits the build (die) if the binary is missing or the call fails, naming the hip-runtime package to install.
fn hipconfig(flag: &str) -> String {
	let out = match process::Command::new("hipconfig").arg(flag).output() {
		Ok(out) => out,
		Err(e) if e.kind() == io::ErrorKind::NotFound => {
			die("hipconfig not found; install hip-runtime-amd or hip-runtime-nvidia")
		}
		Err(e) => die(&format!("hipconfig {flag}: cannot run: {e}")),
	};
	if !out.status.success() {
		die(&format!(
			"hipconfig {flag}: {}",
			String::from_utf8_lossy(&out.stderr)
		));
	}
	return String::from_utf8_lossy(&out.stdout).trim().to_owned();
}

/// Returns the `ROCm` install root as reported by `hipconfig --rocmpath`, the base for every include and link path.
/// Aborts the build (via `die`) if hipconfig is missing, errors, or yields empty output — never returns a fallback.
fn rocm_path() -> String {
	let p = hipconfig("--rocmpath");
	if p.is_empty() {
		die("hipconfig --rocmpath: empty output");
	}
	return p;
}

/// Recursively appends every `.hip` file under `dir` to `out` in filesystem order.
/// An unreadable directory is silently skipped, not an error.
fn collect_hip_files(dir: &Path, out: &mut Vec<PathBuf>) {
	let Ok(entries) = fs::read_dir(dir) else {
		return;
	};
	for entry in entries.flatten() {
		let path = entry.path();
		match Some(()).filter(|_u| return path.is_dir()) {
			Some(()) => collect_hip_files(&path, out),
			None => {
				let hip = Some(path)
					.filter(|p| return p.extension().is_some_and(|e| return e == "hip"));
				out.extend(hip);
			}
		}
	}
}

/// Recursively removes every stale `.o` and `libhip*.a` under `dir` so the full kernel set recompiles fresh, keeping a leftover object from re-linking as a duplicate symbol.
/// Unreadable dirs and failed removals are silently skipped; never fails the build.
fn sweep(dir: &Path) {
	let Ok(rd) = fs::read_dir(dir) else {
		return;
	};
	for e in rd.flatten() {
		let p = e.path();
		match Some(()).filter(|_u| return p.is_dir()) {
			Some(()) => sweep(&p),
			None => {
				let name = p.file_name().and_then(|n| return n.to_str()).unwrap_or("");
				let stale = p.extension().is_some_and(|e| return e == "o")
					|| (name.starts_with("libhip") && name.ends_with(".a"));
				if stale {
					drop(fs::remove_file(&p));
				}
			}
		}
	}
}

fn main() {
	{
		let banned = ["rocblas", "cublas"];
		fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
			let Ok(rd) = fs::read_dir(dir) else {
				return;
			};
			for e in rd.flatten() {
				let p = e.path();
				match Some(()).filter(|_u| return p.is_dir()) {
					Some(()) => walk(&p, out),
					None => {
						let rs = Some(p).filter(|q| {
							return q.extension().is_some_and(|x| return x == "rs");
						});
						out.extend(rs);
					}
				}
			}
		}
		let mut files = Vec::new();
		walk(Path::new("src"), &mut files);
		for f in files {
			let text = fs::read_to_string(&f).unwrap_or_default();
			let lines: Vec<&str> = text.lines().collect();
			for (i, line) in lines.iter().enumerate() {
				let low = line.to_lowercase();
				for pat in &banned {
					if low.contains(pat) {
						die(&format!(
							"{}:{}: direct {} banned — call hipBLAS (hipblas*) instead",
							f.display(),
							i.saturating_add(1),
							pat
						));
					}
				}
			}
		}
	}
	{
		let apis = [
			"hipMemcpyAsync(",
			"hipHostMalloc(",
			"hipMallocAsync(",
			"hipMemsetAsync(",
			"hipHostFree(",
			"hipFreeAsync(",
			"hipMemPoolSetAttribute(",
			"hipMemPoolTrimTo(",
		];
		fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
			let Ok(rd) = fs::read_dir(dir) else {
				return;
			};
			for e in rd.flatten() {
				let p = e.path();
				match Some(()).filter(|_u| return p.is_dir()) {
					Some(()) => walk(&p, out),
					None => {
						let rs = Some(p).filter(|q| {
							return q.extension().is_some_and(|x| return x == "rs");
						});
						out.extend(rs);
					}
				}
			}
		}
		let mut files = Vec::new();
		walk(Path::new("src"), &mut files);
		let mut counts = vec![0usize; apis.len()];
		let mut sites: Vec<Vec<String>> = vec![Vec::new(); apis.len()];
		for f in &files {
			let text = fs::read_to_string(f).unwrap_or_default();
			let lines: Vec<&str> = text.lines().collect();
			for (i, line) in lines.iter().enumerate() {
				for (k, api) in apis.iter().enumerate() {
					if line.contains(api) {
						counts[k] = counts[k].saturating_add(1);
						sites[k].push(format!(
							"{}:{}",
							f.display(),
							i.saturating_add(1)
						));
					}
				}
			}
		}
		for (k, api) in apis.iter().enumerate() {
			if counts[k] > 2 {
				die(&format!(
					"{}: {} occurrences (max 2 = decl + choke call site): {}",
					api,
					counts[k],
					sites[k].join(", ")
				));
			}
		}
	}
	let platform = match hipconfig("--platform").as_str() {
		"amd" => Platform::Amd,
		"nvidia" => Platform::Nvidia,
		other => die(&format!(
			"hipconfig --platform returned {other:?}; install hip-runtime-amd or hip-runtime-nvidia"
		)),
	};
	let out_dir = need(env::var("OUT_DIR").ok(), "OUT_DIR unset");

	let mut hip_files = Vec::new();
	collect_hip_files(Path::new("src/kernels"), &mut hip_files);
	hip_files.sort();
	put("cargo:rerun-if-changed=src/kernels");

	sweep(Path::new(&out_dir));

	match platform {
		Platform::Amd => {
			let rocm = rocm_path();
			let rocm_extra_inc = env::var("ROCM_EXTRA_INCLUDE")
				.unwrap_or_else(|_e| format!("{rocm}/include"));
			let gpu_arch =
				env::var("GPU_ARCH").unwrap_or_else(|_e| return "gfx1101".to_owned());
			let hipcc = env::var("HIPCC").unwrap_or_else(|_e| {
				let hipcc_path = format!("{rocm}/bin/hipcc");
				match Some(hipcc_path).filter(|p| return Path::new(p).exists()) {
					Some(p) => return p,
					None => return format!("{rocm}/bin/amdclang++"),
				}
			});

			cc::Build::new()
				.compiler(&hipcc)
				.no_default_flags(true)
				.warnings(false)
				.flag("-x")
				.flag("hip")
				.flag(format!("--rocm-path={rocm}"))
				.flag(format!("-I{rocm_extra_inc}"))
				.flag("-fPIC")
				.flag(format!("--offload-arch={gpu_arch}"))
				.flag("-O3")
				.files(&hip_files)
				.compile("hipkernels");
		}
		Platform::Nvidia => {
			let rocm = rocm_path();
			let cuda =
				env::var("CUDA_PATH").unwrap_or_else(|_e| return "/opt/cuda".to_owned());
			let hipcc = env::var("HIPCC").unwrap_or_else(|_e| format!("{rocm}/bin/hipcc"));
			let nvcc = env::var("NVCC").unwrap_or_else(|_e| format!("{cuda}/bin/nvcc"));
			let cuda_arch =
				env::var("CUDA_ARCH").unwrap_or_else(|_e| return "sm_86".to_owned());
			let arch_flag = format!("-arch={cuda_arch}");
			let manifest = need(
				env::var("CARGO_MANIFEST_DIR").ok(),
				"CARGO_MANIFEST_DIR unset",
			);
			let compat = format!("{manifest}/src/nvidia_compat");
			let shfl_compat = format!("{compat}/hip_shfl_compat.cuh");

			let nvhip = format!("{out_dir}/nvhip");
			drop(fs::remove_dir_all(&nvhip));
			if let Err(e) = fs::create_dir_all(&nvhip) {
				die(&format!("mkdir {nvhip}: {e}"));
			}
			drop(unix_fs::symlink(
				format!("{rocm}/include/hip"),
				format!("{nvhip}/hip"),
			));

			let cudir = format!("{out_dir}/cu");
			drop(fs::remove_dir_all(&cudir));
			if let Err(e) = fs::create_dir_all(&cudir) {
				die(&format!("mkdir {cudir}: {e}"));
			}
			let mut device_lib_cus = Vec::new();
			let mut plain_cus = Vec::new();
			for src_path in &hip_files {
				let src = need(src_path.to_str(), "non-utf8 kernel path");
				let rel = match src_path.strip_prefix("src/kernels") {
					Ok(r) => r,
					Err(e) => die(&format!("{src}: outside src/kernels: {e}")),
				};
				let cu_name = need(rel.to_str(), "non-utf8 kernel path")
					.replace(['/', '\\', '.'], "_");
				let cu = format!("{cudir}/{cu_name}.cu");
				if let Err(e) = fs::copy(src, &cu) {
					die(&format!("copy {src} to {cu}: {e}"));
				}
				let text = fs::read_to_string(src_path).unwrap_or_default();
				if text.contains("rocprim") || text.contains("hipcub") {
					device_lib_cus.push(cu);
				} else {
					plain_cus.push(cu);
				}
			}

			if !device_lib_cus.is_empty() {
				cc::Build::new()
					.compiler(&nvcc)
					.no_default_flags(true)
					.warnings(false)
					.flag("-x")
					.flag("cu")
					.flag("-O3")
					.flag(&arch_flag)
					.flag("-diag-suppress")
					.flag("2810")
					.flag("-isystem")
					.flag(&compat)
					.flag("-isystem")
					.flag(&nvhip)
					.flag("-include")
					.flag(&shfl_compat)
					.flag("-D__HIP_PLATFORM_NVIDIA__=1")
					.flag("-DTHRUST_IGNORE_CUB_VERSION_CHECK")
					.flag("-Xcompiler")
					.flag("-fPIC")
					.files(&device_lib_cus)
					.compile("hipkernels_devlib");
			}

			if !plain_cus.is_empty() {
				unsafe { env::set_var("HIP_PLATFORM", "nvidia") };
				cc::Build::new()
					.compiler(&hipcc)
					.no_default_flags(true)
					.warnings(false)
					.flag("-fPIC")
					.flag("-O3")
					.flag(&arch_flag)
					.flag("-diag-suppress")
					.flag("2810")
					.flag("-include")
					.flag(&shfl_compat)
					.files(&plain_cus)
					.compile("hipkernels");
				unsafe {
					env::remove_var("HIP_PLATFORM");
				}
			}

			put("cargo:rerun-if-changed=src/shim_nvidia.cu");
			cc::Build::new()
				.compiler(&nvcc)
				.no_default_flags(true)
				.warnings(false)
				.flag("-O3")
				.flag(&arch_flag)
				.flag("-Xcompiler")
				.flag("-fPIC")
				.flag(format!("-I{cuda}/include"))
				.file("src/shim_nvidia.cu")
				.compile("hipshim");
		}
	}

	match platform {
		Platform::Amd => {
			let rocm = rocm_path();
			let rocm_extra_lib =
				env::var("ROCM_EXTRA_LIB").unwrap_or_else(|_e| format!("{rocm}/lib"));
			put(&format!("cargo:rustc-link-search=native={rocm}/lib"));
			put("cargo:rustc-link-lib=dylib=amdhip64");
			put(&format!("cargo:rustc-link-search=native={rocm_extra_lib}"));
			// hipBLAS/hipSOLVER/hipFFT (forward to rocBLAS/rocSOLVER/rocFFT on AMD).
			put("cargo:rustc-link-lib=dylib=hipblas");
			put("cargo:rustc-link-lib=dylib=hipsolver");
			put("cargo:rustc-link-lib=dylib=hipfft");
			put("cargo:rustc-link-lib=dylib=stdc++");
		}
		Platform::Nvidia => {
			let rocm = rocm_path();
			let cuda =
				env::var("CUDA_PATH").unwrap_or_else(|_e| return "/opt/cuda".to_owned());
			// hipBLAS/hipSOLVER/hipFFT built for the NVIDIA platform (wrap cuBLAS/
			// cuSOLVER/cuFFT) live in the HIP install tree, same as on AMD.
			put(&format!("cargo:rustc-link-search=native={rocm}/lib"));
			put("cargo:rustc-link-lib=dylib=hipblas");
			put("cargo:rustc-link-lib=dylib=hipsolver");
			put("cargo:rustc-link-lib=dylib=hipfft");
			put(&format!("cargo:rustc-link-search=native={cuda}/lib64"));
			put("cargo:rustc-link-lib=dylib=cudart");
			put("cargo:rustc-link-lib=dylib=cublas");
			put("cargo:rustc-link-lib=dylib=cusolver");
			put("cargo:rustc-link-lib=dylib=cufft");
			put("cargo:rustc-link-lib=dylib=stdc++");
		}
	}
}
