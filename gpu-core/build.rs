fn main() {
	let rocm = std::env::var("ROCM_PATH").unwrap_or_else(|_| "/opt/rocm".to_string());
	let rocm_extra_lib =
		std::env::var("ROCM_EXTRA_LIB").unwrap_or_else(|_| format!("{rocm}/lib"));
	let rocm_extra_inc =
		std::env::var("ROCM_EXTRA_INCLUDE").unwrap_or_else(|_| format!("{rocm}/include"));
	let gpu_arch = std::env::var("GPU_ARCH").unwrap_or_else(|_| "gfx1101".to_string());
	let hipcc = std::env::var("HIPCC").unwrap_or_else(|_| {
		let hipcc_path = format!("{rocm}/bin/hipcc");
		if std::path::Path::new(&hipcc_path).exists() {
			hipcc_path
		} else {
			format!("{rocm}/bin/amdclang++")
		}
	});
	let out_dir = std::env::var("OUT_DIR").unwrap();

	fn collect_hip_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
		let Ok(entries) = std::fs::read_dir(dir) else {
			return;
		};
		for entry in entries.flatten() {
			let path = entry.path();
			if path.is_dir() {
				collect_hip_files(&path, out);
			} else if path.extension().is_some_and(|e| e == "hip") {
				out.push(path);
			}
		}
	}

	let mut hip_files = Vec::new();
	collect_hip_files(std::path::Path::new("src/kernels"), &mut hip_files);

	let mut objects = Vec::new();
	for src_path in &hip_files {
		let src = src_path.to_str().unwrap();
		let rel = src_path.strip_prefix("src/kernels").unwrap();
		let obj_name = rel.to_str().unwrap().replace(['/', '\\', '.'], "_");
		let obj = format!("{}/{}.o", out_dir, obj_name);
		println!("cargo:rerun-if-changed={}", src);
		let src_mtime = std::fs::metadata(src).and_then(|m| m.modified()).ok();
		let obj_mtime = std::fs::metadata(&obj).and_then(|m| m.modified()).ok();
		let needs_rebuild = match (src_mtime, obj_mtime) {
			(Some(s), Some(o)) => s > o,
			_ => true,
		};
		if needs_rebuild {
			let rocm_path_flag = format!("--rocm-path={rocm}");
			let inc_flag = format!("-I{rocm_extra_inc}");
			let arch_flag = format!("--offload-arch={gpu_arch}");
			let status = std::process::Command::new(&hipcc)
				.args([
					"-x",
					"hip",
					&rocm_path_flag,
					&inc_flag,
					"-c",
					"-fPIC",
					&arch_flag,
					"-O3",
					src,
					"-o",
					&obj,
				])
				.status()
				.expect("hipcc failed");
			assert!(status.success(), "hipcc failed for {}", src);
		}
		objects.push(obj);
	}

	if let Ok(entries) = std::fs::read_dir(&out_dir) {
		for entry in entries.flatten() {
			let p = entry.path();
			if p.to_str().is_some_and(|s| s.ends_with("_hip.o"))
				&& !objects.iter().any(|o| std::path::Path::new(o) == p)
			{
				let _ = std::fs::remove_file(&p);
			}
		}
	}

	if !objects.is_empty() {
		let lib_path = format!("{}/libhipkernels.a", out_dir);
		let _ = std::fs::remove_file(&lib_path);
		let mut ar = std::process::Command::new("ar");
		ar.args(["rcs", &lib_path]);
		for obj in &objects {
			ar.arg(obj);
		}
		ar.status().expect("ar failed");
		println!("cargo:rustc-link-search=native={}", out_dir);
		println!("cargo:rustc-link-lib=static=hipkernels");
	}

	println!("cargo:rustc-link-search=native={rocm}/lib");
	println!("cargo:rustc-link-lib=dylib=amdhip64");
	println!("cargo:rustc-link-search=native={rocm_extra_lib}");
	println!("cargo:rustc-link-lib=dylib=rocblas");
	println!("cargo:rustc-link-lib=dylib=rocsolver");
	println!("cargo:rustc-link-lib=dylib=rocfft");
	println!("cargo:rustc-link-lib=dylib=stdc++");
}
