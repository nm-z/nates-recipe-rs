// Ensure this crate's test/bin artifacts find the from-source hipBLAS-nvidia at
// runtime (the link-arg/rpath from gpu-core's build script does not propagate
// across crates). No-op on AMD.
enum Platform {
	Nvidia,
	Other,
}

fn main() {
	let Platform::Nvidia = platform() else {
		return;
	};
	let hipblas = std::env::var("HIPBLAS_NV_PREFIX").unwrap_or_else(|_e| {
		format!("{}/../gpu-core/vendor/hipblas-nvidia", env!("CARGO_MANIFEST_DIR"))
	});
	println!("cargo:rustc-link-arg=-Wl,-rpath,{hipblas}/lib");
}

fn platform() -> Platform {
	match std::env::var("GPU_PLATFORM") {
		Ok(p) => named_platform(&p),
		Err(_e) => match std::env::var("HIP_PLATFORM") {
			Ok(p) => named_platform(&p),
			Err(_e2) => probe_platform(),
		},
	}
}

fn named_platform(p: &str) -> Platform {
	match p.cmp("nvidia") {
		std::cmp::Ordering::Equal => Platform::Nvidia,
		std::cmp::Ordering::Less | std::cmp::Ordering::Greater => Platform::Other,
	}
}

fn probe_platform() -> Platform {
	let cuda = std::env::var("CUDA_PATH").unwrap_or_else(|_e| "/opt/cuda".to_string());
	match Some(()).filter(|_u| {
		std::path::Path::new(&format!("{cuda}/bin/nvcc")).exists()
			&& std::path::Path::new("/proc/driver/nvidia").exists()
			&& !std::path::Path::new("/sys/module/amdgpu").exists()
			&& !std::path::Path::new("/dev/kfd").exists()
	}) {
		Some(_u) => Platform::Nvidia,
		None => Platform::Other,
	}
}
