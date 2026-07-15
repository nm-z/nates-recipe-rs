// Backend truth comes from hipconfig alone — runtime detection, not hardware:
// no env overrides, no filesystem probes, no hardcoded default. Installing is
// the package manager's job, never the build's: a missing or undecided HIP
// runtime fails the build with the package names.
use std::env;
use std::fs;
use std::io;
use std::process;

enum Platform {
	Amd,
	Nvidia,
}

fn put(s: &str) {
	use std::io::Write as _;
	let mut o = io::stdout();
	drop(o.write_all(s.as_bytes()));
	drop(o.write_all(b"\n"));
}

fn die(s: &str) -> ! {
	use std::io::Write as _;
	let mut e = io::stderr();
	drop(e.write_all(s.as_bytes()));
	drop(e.write_all(b"\n"));
	process::exit(1)
}

fn hipconfig(flag: &str) -> String {
	let out = match process::Command::new("hipconfig").arg(flag).output() {
		Ok(out) => out,
		Err(e) if e.kind() == io::ErrorKind::NotFound => {
			die("hipconfig not found; install hip-runtime-amd or hip-runtime-nvidia")
		}
		Err(e) => die(&format!("hipconfig {flag}: cannot run: {e}")),
	};
	if !out.status.success() {
		die(&format!(
			"hipconfig {flag}: {}",
			String::from_utf8_lossy(&out.stderr)
		));
	}
	String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn detect_platform() -> Platform {
	match hipconfig("--platform").as_str() {
		"amd" => Platform::Amd,
		"nvidia" => Platform::Nvidia,
		other => die(&format!(
			"hipconfig --platform returned {other:?}; install hip-runtime-amd or hip-runtime-nvidia"
		)),
	}
}

fn rocm_path() -> String {
	let p = hipconfig("--rocmpath");
	if p.is_empty() {
		die("hipconfig --rocmpath: empty output");
	}
	p
}

fn main() {
	match detect_platform() {
		Platform::Nvidia => {
			let rocm = rocm_path();
			let cuda =
				env::var("CUDA_PATH").unwrap_or_else(|_e| "/opt/cuda".to_string());
			// hipBLAS/hipSOLVER/hipFFT for the NVIDIA platform (wrap cuBLAS/
			// cuSOLVER/cuFFT) live in the HIP install tree, same as on AMD.
			put(&format!("cargo:rustc-link-search=native={rocm}/lib"));
			put("cargo:rustc-link-lib=dylib=hipblas");
			put("cargo:rustc-link-lib=dylib=hipsolver");
			put("cargo:rustc-link-lib=dylib=hipfft");
			put(&format!("cargo:rustc-link-search=native={cuda}/lib64"));
			put("cargo:rustc-link-lib=dylib=cudart");
			put("cargo:rustc-link-lib=dylib=cublas");
			put("cargo:rustc-link-lib=dylib=cusolver");
			put("cargo:rustc-link-lib=dylib=cufft");
			put("cargo:rustc-link-lib=dylib=stdc++");
		}
		Platform::Amd => {
			let rocm = rocm_path();
			let rocm_extra =
				env::var("ROCM_EXTRA_LIB").unwrap_or_else(|_e| format!("{rocm}/lib"));
			put(&format!("cargo:rustc-link-search=native={rocm}/lib"));
			put("cargo:rustc-link-lib=dylib=amdhip64");
			put(&format!("cargo:rustc-link-search=native={rocm_extra}"));
			// hipBLAS/hipSOLVER/hipFFT (forward to rocBLAS/rocSOLVER/rocFFT on AMD).
			put("cargo:rustc-link-lib=dylib=hipblas");
			put("cargo:rustc-link-lib=dylib=hipsolver");
			put("cargo:rustc-link-lib=dylib=hipfft");
			put("cargo:rustc-link-lib=dylib=stdc++");
		}
	}
	ban_sync_alloc();
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
		let text = fs::read_to_string(&entry).unwrap_or_default();
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
				die(&format!(
					"{}:{}: synchronous {} banned in training crate — use hipMallocAsync/hipFreeAsync",
					entry,
					lineno,
					pat.trim_end_matches('('),
				));
			}
		}
	}
}

fn walkdir(dir: &str) -> Vec<String> {
	let mut out = Vec::new();
	let Ok(rd) = fs::read_dir(dir) else {
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
			put(&format!("cargo:rerun-if-changed={}", f.display()));
		}
	}
	out
}
