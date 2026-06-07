fn main() {
	let rocm = std::env::var("ROCM_PATH").unwrap_or_else(|_| "/opt/rocm".to_string());
	let rocm_extra = std::env::var("ROCM_EXTRA_LIB").unwrap_or_else(|_| format!("{rocm}/lib"));
	println!("cargo:rustc-link-search=native={rocm}/lib");
	println!("cargo:rustc-link-lib=dylib=amdhip64");
	println!("cargo:rustc-link-search=native={rocm_extra}");
	println!("cargo:rustc-link-lib=dylib=rocblas");
	println!("cargo:rustc-link-lib=dylib=rocsolver");
	println!("cargo:rustc-link-lib=dylib=stdc++");
}
