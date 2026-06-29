use std::path::{Path, PathBuf};

// Selects the GPU backend. Honors an explicit GPU_PLATFORM (or HIP_PLATFORM)
// override, otherwise detects the actual hardware/toolchain present so a plain
// `cargo build` works on either an AMD/ROCm box or an NVIDIA/CUDA box. Defaults
// to "amd" (the historical behavior) when nothing conclusive is found.
fn detect_platform() -> String {
	if let Ok(p) = std::env::var("GPU_PLATFORM") {
		return p;
	}
	if let Ok(p) = std::env::var("HIP_PLATFORM") {
		return p;
	}
	let cuda = std::env::var("CUDA_PATH").unwrap_or_else(|_| "/opt/cuda".to_string());
	let have_nvcc = Path::new(&format!("{cuda}/bin/nvcc")).exists();
	let nvidia_gpu = Path::new("/proc/driver/nvidia").exists();
	let amd_gpu = Path::new("/sys/module/amdgpu").exists() || Path::new("/dev/kfd").exists();
	if nvidia_gpu && have_nvcc && !amd_gpu {
		"nvidia".to_string()
	} else {
		"amd".to_string()
	}
}

fn collect_hip_files(dir: &Path, out: &mut Vec<PathBuf>) {
	let Ok(entries) = std::fs::read_dir(dir) else {
		return;
	};
	for entry in entries.flatten() {
		let path = entry.path();
		if path.is_dir() {
			collect_hip_files(&path, out);
		} else if path.extension().is_some_and(|e| e == "hip") {
			out.push(path);
		}
	}
}

fn needs_rebuild(src: &Path, obj: &str) -> bool {
	let src_mtime = std::fs::metadata(src).and_then(|m| m.modified()).ok();
	let obj_mtime = std::fs::metadata(obj).and_then(|m| m.modified()).ok();
	match (src_mtime, obj_mtime) {
		(Some(s), Some(o)) => s > o,
		_ => true,
	}
}

// The framework must call hipBLAS only — direct rocBLAS and cuBLAS are banned in
// gpu-core's Rust sources. cuBLAS lives solely in shim_nvidia.cu (the NVIDIA
// backend, a .cu file), and the inventory test harness in tests/ legitimately
// references the vendor names as data, so only src/*.rs is scanned.
fn ban_direct_blas() {
	let banned = ["rocblas", "cublas"];
	fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
		let Ok(rd) = std::fs::read_dir(dir) else { return };
		for e in rd.flatten() {
			let p = e.path();
			if p.is_dir() {
				walk(&p, out);
			} else if p.extension().is_some_and(|x| x == "rs") {
				out.push(p);
			}
		}
	}
	let mut files = Vec::new();
	walk(Path::new("src"), &mut files);
	for f in files {
		let text = std::fs::read_to_string(&f).unwrap_or_default();
		for (i, line) in text.lines().enumerate() {
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

fn main() {
	ban_direct_blas();
	let platform = detect_platform();
	let out_dir = std::env::var("OUT_DIR").unwrap();

	let mut hip_files = Vec::new();
	collect_hip_files(Path::new("src/kernels"), &mut hip_files);

	let mut objects = Vec::new();

	if platform == "nvidia" {
		build_nvidia(&hip_files, &out_dir, &mut objects);
	} else {
		build_amd(&hip_files, &out_dir, &mut objects);
	}

	// Drop stale kernel/shim objects from previous builds.
	if let Ok(entries) = std::fs::read_dir(&out_dir) {
		for entry in entries.flatten() {
			let p = entry.path();
			if p.to_str().is_some_and(|s| s.ends_with("_hip.o") || s.ends_with("_shim.o"))
				&& !objects.iter().any(|o| Path::new(o) == p)
			{
				let _ = std::fs::remove_file(&p);
			}
		}
	}

	if !objects.is_empty() {
		let lib_path = format!("{}/libhipkernels.a", out_dir);
		let _ = std::fs::remove_file(&lib_path);
		let mut ar = std::process::Command::new("ar");
		ar.args(["rcs", &lib_path]);
		for obj in &objects {
			ar.arg(obj);
		}
		ar.status().expect("ar failed");
		println!("cargo:rustc-link-search=native={}", out_dir);
		println!("cargo:rustc-link-lib=static=hipkernels");
	}

	if platform == "nvidia" {
		link_nvidia();
	} else {
		link_amd();
	}
}

// ── AMD / ROCm backend ─────────────────────────────────────────────────────
fn build_amd(hip_files: &[PathBuf], out_dir: &str, objects: &mut Vec<String>) {
	let rocm = std::env::var("ROCM_PATH").unwrap_or_else(|_| "/opt/rocm".to_string());
	let rocm_extra_inc =
		std::env::var("ROCM_EXTRA_INCLUDE").unwrap_or_else(|_| format!("{rocm}/include"));
	let gpu_arch = std::env::var("GPU_ARCH").unwrap_or_else(|_| "gfx1101".to_string());
	let hipcc = std::env::var("HIPCC").unwrap_or_else(|_| {
		let hipcc_path = format!("{rocm}/bin/hipcc");
		if Path::new(&hipcc_path).exists() {
			hipcc_path
		} else {
			format!("{rocm}/bin/amdclang++")
		}
	});

	for src_path in hip_files {
		let src = src_path.to_str().unwrap();
		let rel = src_path.strip_prefix("src/kernels").unwrap();
		let obj_name = rel.to_str().unwrap().replace(['/', '\\', '.'], "_");
		let obj = format!("{}/{}.o", out_dir, obj_name);
		println!("cargo:rerun-if-changed={}", src);
		if needs_rebuild(src_path, &obj) {
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
	let rocm = std::env::var("ROCM_PATH").unwrap_or_else(|_| "/opt/rocm".to_string());
	let rocm_extra_lib =
		std::env::var("ROCM_EXTRA_LIB").unwrap_or_else(|_| format!("{rocm}/lib"));
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
	let rocm = std::env::var("ROCM_PATH").unwrap_or_else(|_| "/opt/rocm".to_string());
	let cuda = std::env::var("CUDA_PATH").unwrap_or_else(|_| "/opt/cuda".to_string());
	let hipcc = std::env::var("HIPCC").unwrap_or_else(|_| format!("{rocm}/bin/hipcc"));
	let nvcc = std::env::var("NVCC").unwrap_or_else(|_| format!("{cuda}/bin/nvcc"));
	let cuda_arch = std::env::var("CUDA_ARCH").unwrap_or_else(|_| "sm_86".to_string());
	let arch_flag = format!("-arch={cuda_arch}");
	let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
	let compat = format!("{manifest}/src/nvidia_compat");
	let shfl_compat = format!("{compat}/hip_shfl_compat.cuh");

	let nvhip = format!("{out_dir}/nvhip");
	let _ = std::fs::remove_dir_all(&nvhip);
	std::fs::create_dir_all(&nvhip).expect("mkdir nvhip");
	let _ = std::os::unix::fs::symlink(format!("{rocm}/include/hip"), format!("{nvhip}/hip"));

	for src_path in hip_files {
		let src = src_path.to_str().unwrap();
		let rel = src_path.strip_prefix("src/kernels").unwrap();
		let obj_name = rel.to_str().unwrap().replace(['/', '\\', '.'], "_");
		let obj = format!("{}/{}.o", out_dir, obj_name);
		println!("cargo:rerun-if-changed={}", src);
		if needs_rebuild(src_path, &obj) {
			let text = std::fs::read_to_string(src_path).unwrap_or_default();
			let uses_device_lib = text.contains("rocprim") || text.contains("hipcub");
			let cu = format!("{}/{}.cu", out_dir, obj_name);
			std::fs::copy(src, &cu).expect("copy .hip -> .cu failed");
			let status = if uses_device_lib {
				std::process::Command::new(&nvcc)
					.args([
						"-x", "cu", "-c", "-O3", &arch_flag, "-diag-suppress", "2810",
						"-isystem", &compat, "-isystem", &nvhip, "-include", &shfl_compat,
						"-D__HIP_PLATFORM_NVIDIA__=1", "-DTHRUST_IGNORE_CUB_VERSION_CHECK",
						"-Xcompiler", "-fPIC", &cu, "-o", &obj,
					])
					.status()
					.expect("nvcc (nvidia kernel) failed")
			} else {
				std::process::Command::new(&hipcc)
					.env("HIP_PLATFORM", "nvidia")
					.args([
						"-c", "-fPIC", "-O3", &arch_flag, "-diag-suppress", "2810",
						"-include", &shfl_compat, &cu, "-o", &obj,
					])
					.status()
					.expect("hipcc (nvidia) failed")
			};
			assert!(status.success(), "kernel compile failed for {}", src);
		}
		objects.push(obj);
	}

	// HIP host-runtime shim.
	let shim_src = Path::new("src/shim_nvidia.cu");
	let shim_obj = format!("{}/shim_nvidia_shim.o", out_dir);
	println!("cargo:rerun-if-changed=src/shim_nvidia.cu");
	if needs_rebuild(shim_src, &shim_obj) {
		let status = std::process::Command::new(&nvcc)
			.args([
				"-c", "-O3", &arch_flag, "-Xcompiler", "-fPIC",
				&format!("-I{cuda}/include"), "src/shim_nvidia.cu", "-o", &shim_obj,
			])
			.status()
			.expect("nvcc shim failed");
		assert!(status.success(), "nvcc failed for shim_nvidia.cu");
	}
	objects.push(shim_obj);
}

fn link_nvidia() {
	let cuda = std::env::var("CUDA_PATH").unwrap_or_else(|_| "/opt/cuda".to_string());
	let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
	// Real hipBLAS, built from source for HIP_PLATFORM=nvidia (wraps cuBLAS).
	// Override the location with HIPBLAS_NV_PREFIX; default is the vendored build.
	let hipblas = std::env::var("HIPBLAS_NV_PREFIX")
		.unwrap_or_else(|_| format!("{manifest}/vendor/hipblas-nvidia"));
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
