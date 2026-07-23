fn main() {
      let rocm = std::env::var("ROCM_PATH").unwrap_or_else(|_| "/opt/rocm".to_string());
      println!("cargo:rustc-link-search=native={rocm}/lib");
      println!("cargo:rustc-link-lib=dylib=hiprtc");
      println!("cargo:rustc-link-lib=dylib=amd_comgr");
}
