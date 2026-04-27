fn main() {
      let hipcc = if std::path::Path::new("/opt/rocm/bin/hipcc").exists() {
            "/opt/rocm/bin/hipcc"
      } else {
            "/opt/rocm/bin/amdclang++"
      };
      let out_dir = std::env::var("OUT_DIR").unwrap();
      let kernels = ["elementwise", "reduce", "distance", "argsort", "tree", "dtw", "apriori", "lightgbm"];

      let mut objects = Vec::new();
      for name in &kernels {
            let src = format!("src/kernels/{}.hip", name);
            let obj = format!("{}/{}.o", out_dir, name);
            if std::path::Path::new(&src).exists() {
                  println!("cargo:rerun-if-changed={}", src);
                  let src_mtime = std::fs::metadata(&src).and_then(|m| m.modified()).ok();
                  let obj_mtime = std::fs::metadata(&obj).and_then(|m| m.modified()).ok();
                  let needs_rebuild = match (src_mtime, obj_mtime) {
                        (Some(s), Some(o)) => s > o,
                        _ => true,
                  };
                  if needs_rebuild {
                        let status = std::process::Command::new(hipcc)
                              .args(&["-x", "hip", "--rocm-path=/opt/rocm",
                                    "-I/home/nate/.rocm-install/rocm/include",
                                    "-c", "-fPIC", "--offload-arch=gfx1101", "-O3",
                                    &src, "-o", &obj])
                              .status()
                              .expect("hipcc failed");
                        assert!(status.success(), "hipcc failed for {}", src);
                  }
                  objects.push(obj);
            }
      }

      if !objects.is_empty() {
            let lib_path = format!("{}/libhipkernels.a", out_dir);
            let mut ar = std::process::Command::new("ar");
            ar.args(&["rcs", &lib_path]);
            for obj in &objects { ar.arg(obj); }
            ar.status().expect("ar failed");
            println!("cargo:rustc-link-search=native={}", out_dir);
            println!("cargo:rustc-link-lib=static=hipkernels");
      }

      println!("cargo:rustc-link-search=native=/opt/rocm/lib");
      println!("cargo:rustc-link-lib=dylib=amdhip64");
      println!("cargo:rustc-link-search=native=/home/nate/.rocm-install/rocm/lib");
      println!("cargo:rustc-link-lib=dylib=rocblas");
      println!("cargo:rustc-link-lib=dylib=rocsolver");
      println!("cargo:rustc-link-lib=dylib=stdc++");
}
