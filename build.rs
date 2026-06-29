fn main() {
	match detect_platform().as_str() {
		"nvidia" => {
			let cuda = std::env::var("CUDA_PATH").unwrap_or_else(|_| "/opt/cuda".to_string());
			// Real from-source hipBLAS (HIP_PLATFORM=nvidia → cuBLAS). HIPBLAS_NV_PREFIX
			// overrides; default is the vendored build under gpu-core/.
			let hipblas = std::env::var("HIPBLAS_NV_PREFIX").unwrap_or_else(|_| {
				format!("{}/gpu-core/vendor/hipblas-nvidia", env!("CARGO_MANIFEST_DIR"))
			});
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
		_ => {
			let rocm = std::env::var("ROCM_PATH").unwrap_or_else(|_| "/opt/rocm".to_string());
			let rocm_extra = std::env::var("ROCM_EXTRA_LIB").unwrap_or_else(|_| format!("{rocm}/lib"));
			println!("cargo:rustc-link-search=native={rocm}/lib");
			println!("cargo:rustc-link-lib=dylib=amdhip64");
			println!("cargo:rustc-link-search=native={rocm_extra}");
			// hipBLAS/hipSOLVER/hipFFT (forward to rocBLAS/rocSOLVER/rocFFT on AMD).
			println!("cargo:rustc-link-lib=dylib=hipblas");
			println!("cargo:rustc-link-lib=dylib=hipsolver");
			println!("cargo:rustc-link-lib=dylib=hipfft");
			println!("cargo:rustc-link-lib=dylib=stdc++");
		}
	}
	ban_sync_alloc();
}

// Mirrors gpu-core/build.rs: explicit GPU_PLATFORM/HIP_PLATFORM override, else
// detect the hardware/toolchain actually present. Defaults to "amd".
fn detect_platform() -> String {
	if let Ok(p) = std::env::var("GPU_PLATFORM") {
		return p;
	}
	if let Ok(p) = std::env::var("HIP_PLATFORM") {
		return p;
	}
	let cuda = std::env::var("CUDA_PATH").unwrap_or_else(|_| "/opt/cuda".to_string());
	let have_nvcc = std::path::Path::new(&format!("{cuda}/bin/nvcc")).exists();
	let nvidia_gpu = std::path::Path::new("/proc/driver/nvidia").exists();
	let amd_gpu = std::path::Path::new("/sys/module/amdgpu").exists()
		|| std::path::Path::new("/dev/kfd").exists();
	if nvidia_gpu && have_nvcc && !amd_gpu {
		"nvidia".to_string()
	} else {
		"amd".to_string()
	}
}

fn ban_sync_alloc() {
	let banned = ["hipMalloc(", "hipFree("];
	let allowed = ["hipMallocAsync", "hipFreeAsync", "hipMallocManaged", "fn hipMalloc", "fn hipFree"];
	for entry in walkdir("src") {
		let text = std::fs::read_to_string(&entry).unwrap_or_default();
		for (lineno, line) in text.lines().enumerate() {
			let trimmed = line.trim();
			if trimmed.starts_with("//") { continue; }
			for pat in &banned {
				if line.contains(pat) && !allowed.iter().any(|a| line.contains(a)) {
					panic!(
						"{}:{}: synchronous {} banned in training crate — use hipMallocAsync/hipFreeAsync",
						entry, lineno + 1, pat.trim_end_matches('('),
					);
				}
			}
		}
	}
}

fn walkdir(dir: &str) -> Vec<String> {
	let mut out = Vec::new();
	let Ok(rd) = std::fs::read_dir(dir) else { return out; };
	for e in rd.flatten() {
		let p = e.path();
		if p.is_dir() {
			out.extend(walkdir(p.to_str().unwrap_or_default()));
		} else if p.extension().is_some_and(|e| e == "rs") {
			out.push(p.to_string_lossy().into_owned());
			println!("cargo:rerun-if-changed={}", p.display());
		}
	}
	out
}
