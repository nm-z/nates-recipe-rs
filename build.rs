fn main() {
      println!("cargo:rustc-link-search=native=/opt/rocm/lib");
      println!("cargo:rustc-link-lib=dylib=amdhip64");
      println!("cargo:rustc-link-search=native=/home/nate/.rocm-install/rocm/lib");
      println!("cargo:rustc-link-lib=dylib=rocblas");
      println!("cargo:rustc-link-lib=dylib=rocsolver");
      println!("cargo:rustc-link-lib=dylib=stdc++");
}
