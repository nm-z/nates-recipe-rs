fn main() {
	match detect_platform() {
		Platform::Nvidia => {
			let cuda =
				std::env::var("CUDA_PATH").unwrap_or_else(|_e| "/opt/cuda".to_string());
			// Real from-source hipBLAS (HIP_PLATFORM=nvidia → cuBLAS). HIPBLAS_NV_PREFIX
			// overrides; default is the vendored build under gpu-core/.
			let hipblas = std::env::var("HIPBLAS_NV_PREFIX").unwrap_or_else(|_e| {
				format!(
					"{}/gpu-core/vendor/hipblas-nvidia",
					env!("CARGO_MANIFEST_DIR")
				)
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
		Platform::Amd => {
			let rocm =
				std::env::var("ROCM_PATH").unwrap_or_else(|_e| "/opt/rocm".to_string());
			let rocm_extra =
				std::env::var("ROCM_EXTRA_LIB").unwrap_or_else(|_e| format!("{rocm}/lib"));
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

enum Platform {
	Nvidia,
	Amd,
}

// Mirrors gpu-core/build.rs: explicit GPU_PLATFORM/HIP_PLATFORM override, else
// detect the hardware/toolchain actually present. Defaults to "amd".
fn detect_platform() -> Platform {
	let configured = std::env::var("GPU_PLATFORM").or_else(|_e| std::env::var("HIP_PLATFORM"));
	match configured {
		Ok(value) => match Some(value.as_str()).filter(|name| *name == "nvidia") {
			Some(_hit) => Platform::Nvidia,
			None => Platform::Amd,
		},
		Err(_e) => {
			let cuda =
				std::env::var("CUDA_PATH").unwrap_or_else(|_e2| "/opt/cuda".to_string());
			let have_nvcc = std::path::Path::new(&format!("{cuda}/bin/nvcc")).exists();
			let nvidia_gpu = std::path::Path::new("/proc/driver/nvidia").exists();
			let amd_gpu = std::path::Path::new("/sys/module/amdgpu").exists()
				|| std::path::Path::new("/dev/kfd").exists();
			// [nvidia_gpu, have_nvcc, amd_gpu]: nvidia only with an NVIDIA GPU + nvcc and no AMD.
			match Some(())
				.filter(|_u| nvidia_gpu)
				.filter(|_u| have_nvcc)
				.filter(|_u| !amd_gpu)
			{
				Some(()) => Platform::Nvidia,
				None => Platform::Amd,
			}
		}
	}
}

fn ban_sync_alloc() {
	let banned = ["hipMalloc(", "hipFree("];
	let allowed = [
		"hipMallocAsync",
		"hipFreeAsync",
		"hipMallocManaged",
		"fn hipMalloc",
		"fn hipFree",
	];
	for entry in walkdir("src") {
		let text = std::fs::read_to_string(&entry).unwrap_or_default();
		let mut lineno = 0usize;
		for line in text.lines() {
			lineno += 1;
			// skip //-leading comment lines: never a real call site.
			let code = !line.trim().starts_with("//");
			for pat in &banned {
				let hit =
					code && line.contains(pat)
						&& !allowed.iter().any(|a| line.contains(a));
				let Some(_fire) = Some(pat).filter(|_p| hit) else {
					continue;
				};
				panic!(
					"{}:{}: synchronous {} banned in training crate — use hipMallocAsync/hipFreeAsync",
					entry,
					lineno,
					pat.trim_end_matches('('),
				);
			}
		}
	}
}

fn walkdir(dir: &str) -> Vec<String> {
	let mut out = Vec::new();
	let Ok(rd) = std::fs::read_dir(dir) else {
		return out;
	};
	for e in rd.flatten() {
		let p = e.path();
		// directory arm: recurse into it.
		for d in Some(&p).filter(|q| q.is_dir()).into_iter() {
			out.extend(walkdir(d.to_str().unwrap_or_default()));
		}
		// file arm: .rs files register a rerun trigger and join the list.
		for f in Some(&p)
			.filter(|q| !q.is_dir() && q.extension().is_some_and(|e| e == "rs"))
			.into_iter()
		{
			out.push(f.to_string_lossy().into_owned());
			println!("cargo:rerun-if-changed={}", f.display());
		}
	}
	out
}
