use std::path::{Path, PathBuf};

#[derive(Clone, Copy)]
enum Platform {
	Amd,
	Nvidia,
}

// Selects the GPU backend. Honors an explicit GPU_PLATFORM (or HIP_PLATFORM)
// override, otherwise detects the actual hardware/toolchain present so a plain
// `cargo build` works on either an AMD/ROCm box or an NVIDIA/CUDA box. Defaults
// to "amd" (the historical behavior) when nothing conclusive is found.
fn detect_platform() -> Platform {
	let explicit = std::env::var("GPU_PLATFORM")
		.or_else(|_e| std::env::var("HIP_PLATFORM"))
		.ok();
	match explicit {
		Some(p) => match Some(()).filter(|_u| p == "nvidia") {
			Some(()) => Platform::Nvidia,
			None => Platform::Amd,
		},
		None => {
			let cuda =
				std::env::var("CUDA_PATH").unwrap_or_else(|_e| "/opt/cuda".to_string());
			let have_nvcc = Path::new(&format!("{cuda}/bin/nvcc")).exists();
			let nvidia_gpu = Path::new("/proc/driver/nvidia").exists();
			let amd_gpu =
				Path::new("/sys/module/amdgpu").exists() || Path::new("/dev/kfd").exists();
			match Some(()).filter(|_u| nvidia_gpu && have_nvcc && !amd_gpu) {
				Some(()) => Platform::Nvidia,
				None => Platform::Amd,
			}
		}
	}
}

fn collect_hip_files(dir: &Path, out: &mut Vec<PathBuf>) {
	let Ok(entries) = std::fs::read_dir(dir) else {
		return;
	};
	for entry in entries.flatten() {
		let path = entry.path();
		match Some(()).filter(|_u| path.is_dir()) {
			Some(()) => collect_hip_files(&path, out),
			None => {
				let hip = Some(path).filter(|p| p.extension().is_some_and(|e| e == "hip"));
				out.extend(hip);
			}
		}
	}
}

fn needs_rebuild(src: &Path, obj: &str) -> Option<()> {
	let src_mtime = std::fs::metadata(src).and_then(|m| m.modified()).ok();
	let obj_mtime = std::fs::metadata(obj).and_then(|m| m.modified()).ok();
	match src_mtime {
		Some(s) => match obj_mtime {
			Some(o) => Some(()).filter(|_u| s > o),
			None => Some(()),
		},
		None => Some(()),
	}
}

// The framework must call hipBLAS only — direct rocBLAS and cuBLAS are banned in
// gpu-core's Rust sources. cuBLAS lives solely in shim_nvidia.cu (the NVIDIA
// backend, a .cu file), and the inventory test harness in tests/ legitimately
// references the vendor names as data, so only src/*.rs is scanned.
fn ban_direct_blas() {
	let banned = ["rocblas", "cublas"];
	fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
		let Ok(rd) = std::fs::read_dir(dir) else {
			return;
		};
		for e in rd.flatten() {
			let p = e.path();
			match Some(()).filter(|_u| p.is_dir()) {
				Some(()) => walk(&p, out),
				None => {
					let rs = Some(p).filter(|q| q.extension().is_some_and(|x| x == "rs"));
					out.extend(rs);
				}
			}
		}
	}
	let mut files = Vec::new();
	walk(Path::new("src"), &mut files);
	for f in files {
		let text = std::fs::read_to_string(&f).unwrap_or_default();
		let lines: Vec<&str> = text.lines().collect();
		for (i, line) in lines.iter().enumerate() {
			let low = line.to_lowercase();
			for pat in &banned {
				if low.contains(pat) {
					panic!(
						"{}:{}: direct {} banned — call hipBLAS (hipblas*) instead",
						f.display(),
						i + 1,
						pat
					);
				}
			}
		}
	}
}

// Memory ledger law: every living HIP memory API has exactly ONE FFI decl and
// ONE call site (the choke point) in gpu-core's Rust sources — so the byte
// ledger sees every op. Occurrences past 2 (decl + call) break the build.
fn enforce_memory_chokepoints() {
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
		let Ok(rd) = std::fs::read_dir(dir) else {
			return;
		};
		for e in rd.flatten() {
			let p = e.path();
			match Some(()).filter(|_u| p.is_dir()) {
				Some(()) => walk(&p, out),
				None => {
					let rs = Some(p).filter(|q| q.extension().is_some_and(|x| x == "rs"));
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
		let text = std::fs::read_to_string(f).unwrap_or_default();
		let lines: Vec<&str> = text.lines().collect();
		for (i, line) in lines.iter().enumerate() {
			for (k, api) in apis.iter().enumerate() {
				if line.contains(api) {
					counts[k] += 1;
					sites[k].push(format!("{}:{}", f.display(), i + 1));
				}
			}
		}
	}
	for (k, api) in apis.iter().enumerate() {
		if counts[k] > 2 {
			panic!(
				"{}: {} occurrences (max 2 = decl + choke call site): {}",
				api,
				counts[k],
				sites[k].join(", ")
			);
		}
	}
}

fn git(args: &[&str]) -> String {
	let out = std::process::Command::new("git")
		.args(args)
		.output()
		.unwrap_or_else(|e| panic!("git {}: {e}", args.join(" ")));
	assert!(
		out.status.success(),
		"git {}: {}",
		args.join(" "),
		String::from_utf8_lossy(&out.stderr)
	);
	String::from_utf8(out.stdout)
		.expect("git output utf8")
		.trim()
		.to_string()
}

// Bakes the build's git identity for the run-log filename
// (/tmp/recipe/run{hash}.log). rerun-if-changed on HEAD + the checked-out ref
// keeps the bake honest across commits and branch switches; packed-refs covers
// gc'd loose refs. A missing .git is a hard build failure by design.
fn emit_git_hash() {
	println!("cargo:rustc-env=RECIPE_GIT_HASH={}", git(&["rev-parse", "--short", "HEAD"]));
	let gd = git(&["rev-parse", "--absolute-git-dir"]);
	println!("cargo:rerun-if-changed={gd}/HEAD");
	for ref_file in [
		std::process::Command::new("git")
			.args(["symbolic-ref", "-q", "HEAD"])
			.output()
			.ok()
			.filter(|o| o.status.success())
			.map(|o| format!("{gd}/{}", String::from_utf8_lossy(&o.stdout).trim())),
		Some(format!("{gd}/packed-refs")),
	]
	.into_iter()
	.flatten()
	.filter(|p| Path::new(p).exists())
	{
		println!("cargo:rerun-if-changed={ref_file}");
	}
}

fn main() {
	ban_direct_blas();
	enforce_memory_chokepoints();
	emit_git_hash();
	let platform = detect_platform();
	let out_dir = std::env::var("OUT_DIR").unwrap();

	let mut hip_files = Vec::new();
	collect_hip_files(Path::new("src/kernels"), &mut hip_files);

	let mut objects = Vec::new();

	match platform {
		Platform::Nvidia => build_nvidia(&hip_files, &out_dir, &mut objects),
		Platform::Amd => build_amd(&hip_files, &out_dir, &mut objects),
	}

	// Drop stale kernel/shim objects from previous builds.
	for entries in std::fs::read_dir(&out_dir).into_iter() {
		for entry in entries.flatten() {
			let p = entry.path();
			let stale = p
				.to_str()
				.is_some_and(|s| s.ends_with("_hip.o") || s.ends_with("_shim.o"))
				&& !objects.iter().any(|o| Path::new(o) == p);
			if stale {
				drop(std::fs::remove_file(&p));
			}
		}
	}

	if !objects.is_empty() {
		let lib_path = format!("{}/libhipkernels.a", out_dir);
		drop(std::fs::remove_file(&lib_path));
		let mut ar = std::process::Command::new("ar");
		ar.args(["rcs", &lib_path]);
		for obj in &objects {
			ar.arg(obj);
		}
		ar.status().expect("ar failed");
		println!("cargo:rustc-link-search=native={}", out_dir);
		println!("cargo:rustc-link-lib=static=hipkernels");
	}

	match platform {
		Platform::Nvidia => link_nvidia(),
		Platform::Amd => link_amd(),
	}
}

// ── AMD / ROCm backend ─────────────────────────────────────────────────────
fn build_amd(hip_files: &[PathBuf], out_dir: &str, objects: &mut Vec<String>) {
	let rocm = std::env::var("ROCM_PATH").unwrap_or_else(|_e| "/opt/rocm".to_string());
	let rocm_extra_inc =
		std::env::var("ROCM_EXTRA_INCLUDE").unwrap_or_else(|_e| format!("{rocm}/include"));
	let gpu_arch = std::env::var("GPU_ARCH").unwrap_or_else(|_e| "gfx1101".to_string());
	let hipcc = std::env::var("HIPCC").unwrap_or_else(|_e| {
		let hipcc_path = format!("{rocm}/bin/hipcc");
		match Some(hipcc_path).filter(|p| Path::new(p).exists()) {
			Some(p) => p,
			None => format!("{rocm}/bin/amdclang++"),
		}
	});

	for src_path in hip_files {
		let src = src_path.to_str().unwrap();
		let rel = src_path.strip_prefix("src/kernels").unwrap();
		let obj_name = rel.to_str().unwrap().replace(['/', '\\', '.'], "_");
		let obj = format!("{}/{}.o", out_dir, obj_name);
		println!("cargo:rerun-if-changed={}", src);
		for _rebuild in needs_rebuild(src_path, &obj).into_iter() {
			let status = std::process::Command::new(&hipcc)
				.args([
					"-x",
					"hip",
					&format!("--rocm-path={rocm}"),
					&format!("-I{rocm_extra_inc}"),
					"-c",
					"-fPIC",
					&format!("--offload-arch={gpu_arch}"),
					"-O3",
					src,
					"-o",
					&obj,
				])
				.status()
				.expect("hipcc failed");
			assert!(status.success(), "hipcc failed for {}", src);
		}
		objects.push(obj);
	}
}

fn link_amd() {
	let rocm = std::env::var("ROCM_PATH").unwrap_or_else(|_e| "/opt/rocm".to_string());
	let rocm_extra_lib =
		std::env::var("ROCM_EXTRA_LIB").unwrap_or_else(|_e| format!("{rocm}/lib"));
	println!("cargo:rustc-link-search=native={rocm}/lib");
	println!("cargo:rustc-link-lib=dylib=amdhip64");
	println!("cargo:rustc-link-search=native={rocm_extra_lib}");
	// hipBLAS/hipSOLVER/hipFFT (forward to rocBLAS/rocSOLVER/rocFFT on AMD).
	println!("cargo:rustc-link-lib=dylib=hipblas");
	println!("cargo:rustc-link-lib=dylib=hipsolver");
	println!("cargo:rustc-link-lib=dylib=hipfft");
	println!("cargo:rustc-link-lib=dylib=stdc++");
}

// ── NVIDIA / CUDA backend ──────────────────────────────────────────────────
// HIP source compiles unchanged through hipcc with HIP_PLATFORM=nvidia (→ nvcc).
// Files that pull rocPRIM/hipCUB go through plain nvcc + the nvidia_compat shims
// instead (ROCm's bundled CCCL is version-skewed against the system one).
// shim_nvidia.cu supplies the HIP host-runtime symbols.
fn build_nvidia(hip_files: &[PathBuf], out_dir: &str, objects: &mut Vec<String>) {
	let rocm = std::env::var("ROCM_PATH").unwrap_or_else(|_e| "/opt/rocm".to_string());
	let cuda = std::env::var("CUDA_PATH").unwrap_or_else(|_e| "/opt/cuda".to_string());
	let hipcc = std::env::var("HIPCC").unwrap_or_else(|_e| format!("{rocm}/bin/hipcc"));
	let nvcc = std::env::var("NVCC").unwrap_or_else(|_e| format!("{cuda}/bin/nvcc"));
	let cuda_arch = std::env::var("CUDA_ARCH").unwrap_or_else(|_e| "sm_86".to_string());
	let arch_flag = format!("-arch={cuda_arch}");
	let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
	let compat = format!("{manifest}/src/nvidia_compat");
	let shfl_compat = format!("{compat}/hip_shfl_compat.cuh");

	let nvhip = format!("{out_dir}/nvhip");
	drop(std::fs::remove_dir_all(&nvhip));
	std::fs::create_dir_all(&nvhip).expect("mkdir nvhip");
	drop(std::os::unix::fs::symlink(
		format!("{rocm}/include/hip"),
		format!("{nvhip}/hip"),
	));

	for src_path in hip_files {
		let src = src_path.to_str().unwrap();
		let rel = src_path.strip_prefix("src/kernels").unwrap();
		let obj_name = rel.to_str().unwrap().replace(['/', '\\', '.'], "_");
		let obj = format!("{}/{}.o", out_dir, obj_name);
		println!("cargo:rerun-if-changed={}", src);
		for _rebuild in needs_rebuild(src_path, &obj).into_iter() {
			let text = std::fs::read_to_string(src_path).unwrap_or_default();
			let uses_device_lib = text.contains("rocprim") || text.contains("hipcub");
			let cu = format!("{}/{}.cu", out_dir, obj_name);
			std::fs::copy(src, &cu).expect("copy .hip -> .cu failed");
			let status = match Some(()).filter(|_u| uses_device_lib) {
				Some(()) => std::process::Command::new(&nvcc)
					.args([
						"-x",
						"cu",
						"-c",
						"-O3",
						&arch_flag,
						"-diag-suppress",
						"2810",
						"-isystem",
						&compat,
						"-isystem",
						&nvhip,
						"-include",
						&shfl_compat,
						"-D__HIP_PLATFORM_NVIDIA__=1",
						"-DTHRUST_IGNORE_CUB_VERSION_CHECK",
						"-Xcompiler",
						"-fPIC",
						&cu,
						"-o",
						&obj,
					])
					.status()
					.expect("nvcc (nvidia kernel) failed"),
				None => std::process::Command::new(&hipcc)
					.env("HIP_PLATFORM", "nvidia")
					.args([
						"-c",
						"-fPIC",
						"-O3",
						&arch_flag,
						"-diag-suppress",
						"2810",
						"-include",
						&shfl_compat,
						&cu,
						"-o",
						&obj,
					])
					.status()
					.expect("hipcc (nvidia) failed"),
			};
			assert!(status.success(), "kernel compile failed for {}", src);
		}
		objects.push(obj);
	}

	// HIP host-runtime shim.
	let shim_src = Path::new("src/shim_nvidia.cu");
	let shim_obj = format!("{}/shim_nvidia_shim.o", out_dir);
	println!("cargo:rerun-if-changed=src/shim_nvidia.cu");
	for _rebuild in needs_rebuild(shim_src, &shim_obj).into_iter() {
		let status = std::process::Command::new(&nvcc)
			.args([
				"-c",
				"-O3",
				&arch_flag,
				"-Xcompiler",
				"-fPIC",
				&format!("-I{cuda}/include"),
				"src/shim_nvidia.cu",
				"-o",
				&shim_obj,
			])
			.status()
			.expect("nvcc shim failed");
		assert!(status.success(), "nvcc failed for shim_nvidia.cu");
	}
	objects.push(shim_obj);
}

fn link_nvidia() {
	let cuda = std::env::var("CUDA_PATH").unwrap_or_else(|_e| "/opt/cuda".to_string());
	let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
	// Real hipBLAS, built from source for HIP_PLATFORM=nvidia (wraps cuBLAS).
	// Override the location with HIPBLAS_NV_PREFIX; default is the vendored build.
	let hipblas = std::env::var("HIPBLAS_NV_PREFIX")
		.unwrap_or_else(|_e| format!("{manifest}/vendor/hipblas-nvidia"));
	println!("cargo:rustc-link-search=native={hipblas}/lib");
	println!("cargo:rustc-link-arg=-Wl,-rpath,{hipblas}/lib");
	println!("cargo:rustc-link-lib=dylib=hipblas");
	// Vendored from-source hipSOLVER/hipFFT (wrap cuSOLVER/cuFFT); same dir/rpath.
	println!("cargo:rustc-link-lib=dylib=hipsolver");
	println!("cargo:rustc-link-lib=dylib=hipfft");
	println!("cargo:rustc-link-search=native={cuda}/lib64");
	println!("cargo:rustc-link-lib=dylib=cudart");
	println!("cargo:rustc-link-lib=dylib=cublas");
	println!("cargo:rustc-link-lib=dylib=cusolver");
	println!("cargo:rustc-link-lib=dylib=cufft");
	println!("cargo:rustc-link-lib=dylib=stdc++");
}
