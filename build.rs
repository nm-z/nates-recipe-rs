use std::env;
use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::process;

#[derive(Clone, Copy, Debug)]
enum Facade {
	Legacy,
	Next,
}

#[derive(Clone, Copy, Debug)]
enum LegacyPlatform {
	Amd,
	Nvidia,
}

#[derive(Debug)]
enum BuildError {
	FacadeMissing,
	FacadeConflict,
	Legacy(String),
}

impl fmt::Display for BuildError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::FacadeMissing => f.write_str("select exactly one Recipe facade: `legacy` or `next`"),
			Self::FacadeConflict => f.write_str(
				"`legacy` and `next` are mutually exclusive; use \
				 `--no-default-features --features next` for the clean build",
			),
			Self::Legacy(message) => message.fmt(f),
		}
	}
}

impl Error for BuildError {}

fn selected_facade() -> Result<Facade, BuildError> {
	let legacy = env::var_os("CARGO_FEATURE_LEGACY").is_some();
	let next = env::var_os("CARGO_FEATURE_NEXT").is_some();
	match (legacy, next) {
		(true, false) => Ok(Facade::Legacy),
		(false, true) => Ok(Facade::Next),
		(false, false) => Err(BuildError::FacadeMissing),
		(true, true) => Err(BuildError::FacadeConflict),
	}
}

fn source_revision() -> String {
	match env::var_os("RECIPE_GIT_HASH") {
		Some(revision) => revision.to_string_lossy().into_owned(),
		None => "unrecorded".to_owned(),
	}
}

fn put(value: &str) {
	use std::io::Write as _;

	let mut output = io::stdout();
	drop(output.write_all(value.as_bytes()));
	drop(output.write_all(b"\n"));
}

fn legacy_command(flag: &str) -> Result<String, BuildError> {
	let output = match process::Command::new("hipconfig").arg(flag).output() {
		Ok(output) => output,
		Err(error) if error.kind() == io::ErrorKind::NotFound => {
			return Err(BuildError::Legacy(
				"hipconfig not found; install hip-runtime-amd or hip-runtime-nvidia".to_owned(),
			));
		}
		Err(error) => {
			return Err(BuildError::Legacy(format!(
				"hipconfig {flag}: cannot run: {error}"
			)));
		}
	};
	if !output.status.success() {
		return Err(BuildError::Legacy(format!(
			"hipconfig {flag}: {}",
			String::from_utf8_lossy(&output.stderr)
		)));
	}
	Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn legacy_rocm_path() -> Result<String, BuildError> {
	let path = legacy_command("--rocmpath")?;
	if path.is_empty() {
		Err(BuildError::Legacy(
			"hipconfig --rocmpath: empty output".to_owned(),
		))
	} else {
		Ok(path)
	}
}

fn legacy_revision() -> Result<String, BuildError> {
	let hash = process::Command::new("git")
		.args(["rev-parse", "--short", "HEAD"])
		.output()
		.ok()
		.filter(|output| output.status.success())
		.map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
		.ok_or_else(|| BuildError::Legacy("git rev-parse --short HEAD failed".to_owned()))?;
	let dirty = process::Command::new("git")
		.args(["status", "--porcelain"])
		.output()
		.ok()
		.filter(|output| output.status.success())
		.map(|output| !output.stdout.is_empty())
		.ok_or_else(|| BuildError::Legacy("git status --porcelain failed".to_owned()))?;
	Ok(format!("{hash}{}", if dirty { "-dirty" } else { "" }))
}

fn legacy_build() -> Result<(), BuildError> {
	put(&format!("cargo:rustc-env=GIT_HASH={}", legacy_revision()?));
	put("cargo:rerun-if-changed=.git/HEAD");
	let platform = match legacy_command("--platform")?.as_str() {
		"amd" => LegacyPlatform::Amd,
		"nvidia" => LegacyPlatform::Nvidia,
		other => {
			return Err(BuildError::Legacy(format!(
				"hipconfig --platform returned {other:?}; install hip-runtime-amd or hip-runtime-nvidia"
			)));
		}
	};
	match platform {
		LegacyPlatform::Nvidia => {
			let rocm = legacy_rocm_path()?;
			let cuda = env::var("CUDA_PATH").unwrap_or_else(|_| "/opt/cuda".to_owned());
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
		LegacyPlatform::Amd => {
			let rocm = legacy_rocm_path()?;
			let rocm_extra = env::var("ROCM_EXTRA_LIB").unwrap_or_else(|_| format!("{rocm}/lib"));
			put(&format!("cargo:rustc-link-search=native={rocm}/lib"));
			put("cargo:rustc-link-lib=dylib=amdhip64");
			put(&format!("cargo:rustc-link-search=native={rocm_extra}"));
			put("cargo:rustc-link-lib=dylib=hipblas");
			put("cargo:rustc-link-lib=dylib=hipsolver");
			put("cargo:rustc-link-lib=dylib=hipfft");
			put("cargo:rustc-link-lib=dylib=stdc++");
		}
	}
	legacy_source_audit("src")
}

fn legacy_source_audit(directory: &str) -> Result<(), BuildError> {
	let banned = ["hipMalloc(", "hipFree("];
	let allowed = [
		"hipMallocAsync",
		"hipFreeAsync",
		"hipMallocManaged",
		"fn hipMalloc",
		"fn hipFree",
	];
	for entry in walkdir(directory) {
		let text = fs::read_to_string(&entry).unwrap_or_default();
		for (index, line) in text.lines().enumerate() {
			let code = !line.trim().starts_with("//");
			for pattern in &banned {
				let permitted = allowed.iter().any(|allowed| line.contains(allowed));
				if code && line.contains(pattern) && !permitted {
					return Err(BuildError::Legacy(format!(
						"{}:{}: synchronous {} banned in training crate - use hipMallocAsync/hipFreeAsync",
						entry,
						index + 1,
						pattern.trim_end_matches('('),
					)));
				}
			}
		}
	}
	Ok(())
}

fn walkdir(directory: &str) -> Vec<String> {
	let mut entries = Vec::new();
	let Ok(read_dir) = fs::read_dir(directory) else {
		return entries;
	};
	for entry in read_dir.flatten() {
		let path = entry.path();
		if path.is_dir() {
			entries.extend(walkdir(path.to_str().unwrap_or_default()));
		} else if path.extension().is_some_and(|extension| extension == "rs") {
			entries.push(path.to_string_lossy().into_owned());
			put(&format!("cargo:rerun-if-changed={}", path.display()));
		}
	}
	entries
}

fn main() -> Result<(), BuildError> {
	let facade = selected_facade()?;
	match facade {
		Facade::Legacy => {
			println!("cargo:rustc-cfg=recipe_facade_legacy");
			legacy_build()?;
		}
		Facade::Next => {
			println!("cargo:rustc-cfg=recipe_facade_next");
			println!("cargo:rustc-env=GIT_HASH={}", source_revision());
		}
	}
	Ok(())
}
