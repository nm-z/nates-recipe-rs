fn main() {
    let src_files: Vec<String> = std::fs::read_dir("src")
        .expect("can't read src/")
        .chain(std::fs::read_dir("src/utils").unwrap_or_else(|_| std::fs::read_dir("src").unwrap()))
        .chain(std::fs::read_dir("src/gpu").unwrap_or_else(|_| std::fs::read_dir("src").unwrap()))
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
        .map(|e| e.path().to_string_lossy().to_string())
        .collect();
    for path in &src_files {
        println!("cargo:rerun-if-changed={}", path);
    }

    // Build hash: FNV-1a over all source file contents — changes on every recompile.
    let mut h: u64 = 14695981039346656037;
    for path in &src_files {
        if let Ok(src) = std::fs::read(path) {
            for b in src {
                h ^= b as u64;
                h = h.wrapping_mul(1099511628211);
            }
        }
    }
    println!("cargo:rustc-env=BUILD_HASH={:07x}", h & 0xfffffff);
    {
        let hipcc = if std::path::Path::new("/opt/rocm/bin/hipcc").exists() {
            "/opt/rocm/bin/hipcc"
        } else {
            "/opt/rocm/bin/amdclang++"
        };
        let out_dir = std::env::var("OUT_DIR").unwrap();
        let kernels = ["elementwise", "reduce", "distance", "argsort"];

        if !std::path::Path::new(hipcc).exists() {
            eprintln!("cargo:warning=hipcc not found at {}; skipping GPU kernel compilation", hipcc);
            return;
        }

        let mut objects = Vec::new();
        for name in &kernels {
            let src = format!("src/gpu/kernels/{}.hip", name);
            let obj = format!("{}/{}.o", out_dir, name);
            if std::path::Path::new(&src).exists() {
                let status = std::process::Command::new(hipcc)
                    .args(&["-x", "hip", "--rocm-path=/opt/rocm", "-I/home/nate/.rocm-install/rocm/include", "-c", "-fPIC", "--offload-arch=gfx1101", "-O3", &src, "-o", &obj])
                    .status()
                    .expect("hipcc failed — is ROCm installed?");
                assert!(status.success(), "hipcc failed for {}", src);
                objects.push(obj);
                println!("cargo:rerun-if-changed={}", src);
            }
        }

        if !objects.is_empty() {
            let lib_path = format!("{}/libhipkernels.a", out_dir);
            let mut ar = std::process::Command::new("ar");
            ar.args(&["rcs", &lib_path]);
            for obj in &objects {
                ar.arg(obj);
            }
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
}
