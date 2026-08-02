use std::{
	error::Error,
	fs,
	path::{Path, PathBuf},
	process::Command,
};

use sha2::{Digest as _, Sha256};

fn main() -> Result<(), Box<dyn Error>> {
	let manifest = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").ok_or("CARGO_MANIFEST_DIR is unavailable")?);
	let inputs = [
		("recipe-native-probe", manifest.join("src")),
		("recipe-core", manifest.join("../recipe-core/src")),
		("recipe-executor", manifest.join("../recipe-executor/src")),
		("recipe-host", manifest.join("../recipe-host/src")),
		("recipe-kernel", manifest.join("../recipe-kernel/src")),
		("recipe-language", manifest.join("../recipe-language/src")),
		(
			"recipe-native-executor",
			manifest.join("../recipe-native-executor/src"),
		),
		("recipe-planner", manifest.join("../recipe-planner/src")),
		(
			"recipe-primitives",
			manifest.join("../recipe-primitives/src"),
		),
		("recipe-scheduler", manifest.join("../recipe-scheduler/src")),
		("recipe-cuda", manifest.join("../recipe-cuda/src")),
		("recipe-hsa", manifest.join("../recipe-hsa/src")),
		("recipe-probe", manifest.join("../recipe-probe/src")),
	];
	let mut files = Vec::new();
	for (domain, root) in inputs {
		collect_rust_files(domain, &root, &root, &mut files)?;
	}
	files.push((
		"recipe-native-probe/build.rs".to_owned(),
		manifest.join("build.rs"),
	));
	for (name, path) in [
		("Cargo.lock", manifest.join("../Cargo.lock")),
		("Cargo.toml", manifest.join("../Cargo.toml")),
		(
			"recipe-native-probe/Cargo.toml",
			manifest.join("Cargo.toml"),
		),
		(
			"recipe-core/Cargo.toml",
			manifest.join("../recipe-core/Cargo.toml"),
		),
		(
			"recipe-executor/Cargo.toml",
			manifest.join("../recipe-executor/Cargo.toml"),
		),
		(
			"recipe-host/Cargo.toml",
			manifest.join("../recipe-host/Cargo.toml"),
		),
		(
			"recipe-kernel/Cargo.toml",
			manifest.join("../recipe-kernel/Cargo.toml"),
		),
		(
			"recipe-language/Cargo.toml",
			manifest.join("../recipe-language/Cargo.toml"),
		),
		(
			"recipe-native-executor/Cargo.toml",
			manifest.join("../recipe-native-executor/Cargo.toml"),
		),
		(
			"recipe-planner/Cargo.toml",
			manifest.join("../recipe-planner/Cargo.toml"),
		),
		(
			"recipe-primitives/Cargo.toml",
			manifest.join("../recipe-primitives/Cargo.toml"),
		),
		(
			"recipe-scheduler/Cargo.toml",
			manifest.join("../recipe-scheduler/Cargo.toml"),
		),
		(
			"recipe-cuda/Cargo.toml",
			manifest.join("../recipe-cuda/Cargo.toml"),
		),
		(
			"recipe-hsa/Cargo.toml",
			manifest.join("../recipe-hsa/Cargo.toml"),
		),
		(
			"recipe-probe/Cargo.toml",
			manifest.join("../recipe-probe/Cargo.toml"),
		),
	] {
		files.push((name.to_owned(), path));
	}
	files.sort_by(|left, right| left.0.cmp(&right.0));

	let mut digest = Sha256::new();
	digest.update(b"recipe-native-probe-build-v3");
	for (name, path) in files {
		println!("cargo:rerun-if-changed={}", path.display());
		let bytes = fs::read(&path)?;
		hash_field(&mut digest, name.as_bytes());
		hash_field(&mut digest, &bytes);
	}
	for name in [
		"HOST",
		"TARGET",
		"OPT_LEVEL",
		"DEBUG",
		"PROFILE",
		"CARGO_CFG_TARGET_ARCH",
		"CARGO_CFG_TARGET_OS",
		"CARGO_CFG_TARGET_ENV",
		"CARGO_CFG_TARGET_FEATURE",
		"CARGO_ENCODED_RUSTFLAGS",
		"RUSTFLAGS",
	] {
		println!("cargo:rerun-if-env-changed={name}");
		hash_field(&mut digest, name.as_bytes());
		match std::env::var_os(name) {
			Some(value) => hash_field(&mut digest, value.as_encoded_bytes()),
			None => hash_field(&mut digest, &[]),
		}
	}
	hash_rustc_identity(&mut digest)?;
	let source_digest = digest
		.finalize()
		.iter()
		.map(|byte| format!("{byte:02x}"))
		.collect::<String>();
	println!("cargo:rustc-env=RECIPE_NATIVE_PROBE_SOURCE_DIGEST={source_digest}");
	Ok(())
}

fn hash_field(digest: &mut Sha256, value: &[u8]) {
	digest.update((value.len() as u64).to_le_bytes());
	digest.update(value);
}

fn hash_rustc_identity(digest: &mut Sha256) -> Result<(), Box<dyn Error>> {
	println!("cargo:rerun-if-env-changed=RUSTC");
	let rustc = std::env::var_os("RUSTC").ok_or("RUSTC is unavailable")?;
	let output = Command::new(&rustc).arg("-Vv").output()?;
	if !output.status.success() {
		return Err(format!(
			"{} -Vv failed with {}",
			Path::new(&rustc).display(),
			output.status
		)
		.into());
	}
	hash_field(digest, b"RUSTC");
	hash_field(digest, rustc.as_encoded_bytes());
	hash_field(digest, &output.stdout);
	hash_field(digest, &output.stderr);
	Ok(())
}

fn collect_rust_files(
	domain: &str,
	root: &Path,
	directory: &Path,
	output: &mut Vec<(String, PathBuf)>,
) -> Result<(), Box<dyn Error>> {
	let mut entries = fs::read_dir(directory)?.collect::<Result<Vec<_>, _>>()?;
	entries.sort_by_key(|entry| entry.file_name());
	for entry in entries {
		let path = entry.path();
		if path.is_dir() {
			collect_rust_files(domain, root, &path, output)?;
		} else if path.extension().is_some_and(|extension| extension == "rs") {
			let relative = path.strip_prefix(root)?;
			output.push((format!("{domain}/{}", relative.display()), path));
		}
	}
	Ok(())
}
