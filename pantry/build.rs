// Ensure this crate's test/bin artifacts find the from-source hipBLAS-nvidia at
// runtime (the link-arg/rpath from gpu-core's build script does not propagate
// across crates). No-op on AMD.
fn main() {
	for hipblas in nvidia_rpath().into_iter() {
		println!("cargo:rustc-link-arg=-Wl,-rpath,{hipblas}/lib");
	}
}

fn nvidia_rpath() -> Option<String> {
	is_nvidia().map(|()| hipblas_prefix())
}

fn hipblas_prefix() -> String {
	std::env::var("HIPBLAS_NV_PREFIX").unwrap_or_else(|_e| {
		format!(
			"{}/../gpu-core/vendor/hipblas-nvidia",
			env!("CARGO_MANIFEST_DIR")
		)
	})
}

fn is_nvidia() -> Option<()> {
	let forced = std::env::var("GPU_PLATFORM")
		.ok()
		.or_else(|| std::env::var("HIP_PLATFORM").ok());
	match forced {
		Some(p) => nvidia_from_str(&p),
		None => probe_nvidia(),
	}
}

fn nvidia_from_str(p: &str) -> Option<()> {
	Some(()).filter(|_u| p == "nvidia")
}

fn probe_nvidia() -> Option<()> {
	let cuda = std::env::var("CUDA_PATH").unwrap_or_else(|_e| "/opt/cuda".to_string());
	let nvcc = std::path::Path::new(&format!("{cuda}/bin/nvcc")).exists();
	let driver = std::path::Path::new("/proc/driver/nvidia").exists();
	let amdgpu = std::path::Path::new("/sys/module/amdgpu").exists();
	let kfd = std::path::Path::new("/dev/kfd").exists();
	Some(()).filter(|_u| nvcc && driver && !amdgpu && !kfd)
}
