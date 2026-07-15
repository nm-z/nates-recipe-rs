use std::env;
use std::fs;
use std::io;
use std::process;

/// The HIP backend platform, decided solely by `hipconfig --platform`.
enum Platform {
	/// AMD `ROCm`: hipBLAS/hipSOLVER/hipFFT forward to rocBLAS/rocSOLVER/rocFFT.
	Amd,
	/// NVIDIA CUDA: hipBLAS/hipSOLVER/hipFFT wrap cuBLAS/cuSOLVER/cuFFT.
	Nvidia,
}

/// Writes one cargo build directive to stdout, newline-terminated.
/// Write errors are discarded — cargo reads this line-by-line and a dropped byte is not worth failing the build.
fn put(s: &str) {
	use std::io::Write as _;
	let mut o = io::stdout();
	drop(o.write_all(s.as_bytes()));
	drop(o.write_all(b"\n"));
}

/// Runs `hipconfig <flag>` and returns its trimmed stdout — the sole backend-truth source.
/// Errs (naming the packages to install) if hipconfig is missing, fails to run, or returns nonzero.
fn hipconfig(flag: &str) -> Result<String, String> {
	let out = match process::Command::new("hipconfig").arg(flag).output() {
		Ok(out) => out,
		Err(e) if e.kind() == io::ErrorKind::NotFound => {
			return Err(
				"hipconfig not found; install hip-runtime-amd or hip-runtime-nvidia"
					.to_owned(),
			);
		}
		Err(e) => return Err(format!("hipconfig {flag}: cannot run: {e}")),
	};
	if !out.status.success() {
		return Err(format!(
			"hipconfig {flag}: {}",
			String::from_utf8_lossy(&out.stderr)
		));
	}
	return Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned());
}

/// Returns the `ROCm` install tree path queried from `hipconfig --rocmpath`.
/// Errs if hipconfig fails or yields empty output.
fn rocm_path() -> Result<String, String> {
	let p = hipconfig("--rocmpath")?;
	if p.is_empty() {
		return Err("hipconfig --rocmpath: empty output".to_owned());
	}
	return Ok(p);
}

fn main() -> Result<(), String> {
	let platform = match hipconfig("--platform")?.as_str() {
		"amd" => Platform::Amd,
		"nvidia" => Platform::Nvidia,
		other => {
			return Err(format!(
				"hipconfig --platform returned {other:?}; install hip-runtime-amd or hip-runtime-nvidia"
			));
		}
	};
	match platform {
		Platform::Nvidia => {
			let rocm = rocm_path()?;
			let cuda =
				env::var("CUDA_PATH").unwrap_or_else(|_e| return "/opt/cuda".to_owned());
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
			let rocm = rocm_path()?;
			let rocm_extra =
				env::var("ROCM_EXTRA_LIB").unwrap_or_else(|_e| format!("{rocm}/lib"));
			put(&format!("cargo:rustc-link-search=native={rocm}/lib"));
			put("cargo:rustc-link-lib=dylib=amdhip64");
			put(&format!("cargo:rustc-link-search=native={rocm_extra}"));
			put("cargo:rustc-link-lib=dylib=hipblas");
			put("cargo:rustc-link-lib=dylib=hipsolver");
			put("cargo:rustc-link-lib=dylib=hipfft");
			put("cargo:rustc-link-lib=dylib=stdc++");
		}
	}
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
			lineno = lineno.saturating_add(1);
			let code = !line.trim().starts_with("//");
			for pat in &banned {
				let hit =
					code && line.contains(pat)
						&& !allowed.iter().any(|a| return line.contains(a));
				if !hit {
					continue;
				}
				return Err(format!(
					"{}:{}: synchronous {} banned in training crate — use hipMallocAsync/hipFreeAsync",
					entry,
					lineno,
					pat.trim_end_matches('('),
				));
			}
		}
	}
	return Ok(());
}

/// Recursively collects every `.rs` path under `dir`, emitting a cargo:rerun-if-changed for each.
/// An unreadable directory yields no entries rather than failing.
fn walkdir(dir: &str) -> Vec<String> {
	let mut out = Vec::new();
	let Ok(rd) = fs::read_dir(dir) else {
		return out;
	};
	for e in rd.flatten() {
		let p = e.path();
		if p.is_dir() {
			out.extend(walkdir(p.to_str().unwrap_or_default()));
			continue;
		}
		if p.extension().is_some_and(|ext| return ext == "rs") {
			out.push(p.to_string_lossy().into_owned());
			put(&format!("cargo:rerun-if-changed={}", p.display()));
		}
	}
	return out;
}
