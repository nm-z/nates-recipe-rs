// Ensure this crate's test/bin artifacts find the from-source hipBLAS-nvidia at
// runtime (the link-arg/rpath from gpu-core's build script does not propagate
// across crates). No-op on AMD.
fn main() {
	if !is_nvidia() {
		return;
	}
	let hipblas = std::env::var("HIPBLAS_NV_PREFIX").unwrap_or_else(|_| {
		format!("{}/../gpu-core/vendor/hipblas-nvidia", env!("CARGO_MANIFEST_DIR"))
	});
	println!("cargo:rustc-link-arg=-Wl,-rpath,{hipblas}/lib");
}

fn is_nvidia() -> bool {
	if let Ok(p) = std::env::var("GPU_PLATFORM") {
		return p == "nvidia";
	}
	if let Ok(p) = std::env::var("HIP_PLATFORM") {
		return p == "nvidia";
	}
	let cuda = std::env::var("CUDA_PATH").unwrap_or_else(|_| "/opt/cuda".to_string());
	std::path::Path::new(&format!("{cuda}/bin/nvcc")).exists()
		&& std::path::Path::new("/proc/driver/nvidia").exists()
		&& !std::path::Path::new("/sys/module/amdgpu").exists()
		&& !std::path::Path::new("/dev/kfd").exists()
}
