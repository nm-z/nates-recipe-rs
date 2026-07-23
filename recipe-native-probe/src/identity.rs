use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use recipe_core::{Digest as CoreDigest, Label, ToolchainIdentity};
use recipe_kernel::{ArtifactDigest, OfflineToolchain, PinnedTool};
use recipe_probe::{ProbeError, ProbeResult};
use sha2::{Digest as _, Sha256};

use crate::config::BackendLibrary;

const PROBE_SOURCE_DIGEST: &str = env!("RECIPE_NATIVE_PROBE_SOURCE_DIGEST");

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PinnedLibrary {
	pub path: PathBuf,
	pub digest: [u8; 32],
}

pub(crate) fn selected_library(library: &BackendLibrary, backend: &str) -> ProbeResult<Option<PinnedLibrary>> {
	let mut seen = BTreeSet::new();
	let mut selected = None;
	for candidate in &library.candidates {
		if !candidate.is_absolute() {
			return Err(ProbeError::Discovery(format!(
				"{backend} library candidate {} is not absolute",
				candidate.display()
			)));
		}
		let metadata = match fs::symlink_metadata(candidate) {
			Ok(metadata) => metadata,
			Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
			Err(error) => {
				return Err(ProbeError::io(
					"inspect native backend library",
					candidate,
					error,
				));
			}
		};
		if !(metadata.file_type().is_file() || metadata.file_type().is_symlink()) {
			return Err(ProbeError::Discovery(format!(
				"{backend} library candidate {} is not a regular file or symlink",
				candidate.display()
			)));
		}
		let canonical = fs::canonicalize(candidate)
			.map_err(|error| ProbeError::io("canonicalize native backend library", candidate, error))?;
		if !seen.insert(canonical.clone()) {
			continue;
		}
		let canonical_metadata = fs::metadata(&canonical)
			.map_err(|error| ProbeError::io("inspect native backend library target", &canonical, error))?;
		if !canonical_metadata.is_file() {
			return Err(ProbeError::Discovery(format!(
				"{backend} library target {} is not a regular file",
				canonical.display()
			)));
		}
		let bytes = fs::read(&canonical)
			.map_err(|error| ProbeError::io("read native backend library", &canonical, error))?;
		let pinned = PinnedLibrary {
			path: canonical,
			digest: Sha256::digest(bytes).into(),
		};
		if selected.is_none() {
			selected = Some(pinned);
		}
	}
	Ok(selected)
}

pub(crate) fn label(value: impl Into<String>, field: &str) -> ProbeResult<Label> {
	Label::new(value).map_err(|error| ProbeError::Discovery(format!("{field}: {error}")))
}

pub(crate) fn hex(bytes: &[u8]) -> String {
	let mut output = String::with_capacity(bytes.len() * 2);
	for byte in bytes {
		use std::fmt::Write as _;
		write!(output, "{byte:02x}").expect("writing to an in-memory String cannot fail");
	}
	output
}

pub(crate) fn backend_toolchain_identity(
	toolchain: &OfflineToolchain,
	release: &Label,
	backend: &str,
	target_configuration: &str,
) -> ProbeResult<ToolchainIdentity> {
	let mut digest = Sha256::new();
	digest.update(b"recipe-native-probe-toolchain-and-benchmark-v2");
	digest.update((backend.len() as u64).to_le_bytes());
	digest.update(backend.as_bytes());
	digest.update((release.as_str().len() as u64).to_le_bytes());
	digest.update(release.as_str().as_bytes());
	digest.update((target_configuration.len() as u64).to_le_bytes());
	digest.update(target_configuration.as_bytes());
	digest.update((PROBE_SOURCE_DIGEST.len() as u64).to_le_bytes());
	digest.update(PROBE_SOURCE_DIGEST.as_bytes());
	for tool in required_tools(toolchain, backend)? {
		hash_tool(&mut digest, tool);
	}
	let bytes: [u8; 32] = digest.finalize().into();
	Ok(ToolchainIdentity {
		name: label(format!("recipe-owned-llvm-{backend}"), "toolchain name")?,
		version: release.clone(),
		digest: CoreDigest::new(bytes),
	})
}

fn required_tools<'a>(toolchain: &'a OfflineToolchain, backend: &str) -> ProbeResult<Vec<&'a PinnedTool>> {
	let mut tools = vec![&toolchain.verifier, &toolchain.llvm_codegen];
	match backend {
		"amd-hsa" => tools.push(toolchain.elf_linker.as_ref().ok_or_else(|| {
			ProbeError::Discovery("AMD probing requires an explicitly pinned ELF linker".to_owned())
		})?),
		"nvidia-cuda" => tools.push(toolchain.ptx_assembler.as_ref().ok_or_else(|| {
			ProbeError::Discovery("NVIDIA probing requires an explicitly pinned PTX assembler".to_owned())
		})?),
		_ => {
			return Err(ProbeError::Discovery(format!(
				"unknown native toolchain backend {backend}"
			)));
		}
	}
	Ok(tools)
}

fn hash_tool(digest: &mut Sha256, tool: &PinnedTool) {
	let path = tool.path.as_os_str().as_encoded_bytes();
	digest.update((path.len() as u64).to_le_bytes());
	digest.update(path);
	digest.update(artifact_digest_bytes(tool.digest));
}

fn artifact_digest_bytes(digest: ArtifactDigest) -> [u8; 32] {
	let text = digest.to_hex();
	let mut bytes = [0_u8; 32];
	for (index, chunk) in text.as_bytes().chunks_exact(2).enumerate() {
		bytes[index] = hex_nibble(chunk[0]) * 16 + hex_nibble(chunk[1]);
	}
	bytes
}

fn hex_nibble(value: u8) -> u8 {
	match value {
		b'0'..=b'9' => value - b'0',
		b'a'..=b'f' => value - b'a' + 10,
		b'A'..=b'F' => value - b'A' + 10,
		_ => unreachable!("ArtifactDigest::to_hex returns hexadecimal"),
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PciSurface {
	pub driver: String,
	pub firmware: String,
	pub link: String,
}

/// Report whether sysfs contains a GPU/accelerator PCI function for `vendor`.
///
/// This is a preflight for optional native runtimes: an installed ROCr
/// library on a machine with no AMD accelerator is not evidence that the HSA
/// backend participates. Once matching hardware exists, runtime failures
/// remain fatal and cannot be silently downgraded to backend absence.
pub(crate) fn pci_accelerator_present(root: &Path, vendor: u16) -> ProbeResult<bool> {
	if !root.is_absolute() {
		return Err(ProbeError::Discovery(format!(
			"PCI sysfs root {} is not absolute",
			root.display()
		)));
	}
	let entries = fs::read_dir(root).map_err(|error| ProbeError::io("enumerate PCI devices", root, error))?;
	for entry in entries {
		let entry = entry.map_err(|error| ProbeError::io("enumerate PCI device", root, error))?;
		let path = entry.path();
		let actual_vendor = read_pci_hex(&path.join("vendor"), "vendor")?;
		let class = read_pci_hex(&path.join("class"), "class")?;
		if actual_vendor == u32::from(vendor) && is_accelerator_class(class) {
			return Ok(true);
		}
	}
	Ok(false)
}

fn read_pci_hex(path: &Path, field: &str) -> ProbeResult<u32> {
	let text = fs::read_to_string(path).map_err(|error| ProbeError::io("read PCI identity", path, error))?;
	let digits = text.trim().strip_prefix("0x").unwrap_or(text.trim());
	u32::from_str_radix(digits, 16).map_err(|error| {
		ProbeError::Discovery(format!(
			"PCI {field} at {} is not hexadecimal: {error}",
			path.display()
		))
	})
}

const fn is_accelerator_class(class: u32) -> bool {
	matches!(class >> 16, 0x03 | 0x12)
}

pub(crate) fn pci_surface(root: &Path, bdf: &str) -> ProbeResult<PciSurface> {
	if !root.is_absolute() {
		return Err(ProbeError::Discovery(format!(
			"PCI sysfs root {} is not absolute",
			root.display()
		)));
	}
	let device_root = root.join(bdf);
	let metadata =
		fs::metadata(&device_root).map_err(|error| ProbeError::io("inspect PCI device", &device_root, error))?;
	if !metadata.is_dir() {
		return Err(ProbeError::Discovery(format!(
			"PCI device path {} is not a directory",
			device_root.display()
		)));
	}

	let driver_target = fs::canonicalize(device_root.join("driver"))
		.map_err(|error| ProbeError::io("resolve PCI driver", device_root.join("driver"), error))?;
	let driver_files = [
		PathBuf::from("/proc/sys/kernel/osrelease"),
		device_root.join("driver/module/version"),
	];
	let driver = surface_digest(
		"driver",
		Some(driver_target.as_os_str().as_encoded_bytes()),
		driver_files.iter(),
		true,
	)?;
	let firmware_files = [
		device_root.join("revision"),
		device_root.join("subsystem_vendor"),
		device_root.join("subsystem_device"),
		device_root.join("vbios_version"),
	];
	let firmware = surface_digest("firmware", None, firmware_files.iter(), false)?;
	let link_files = [
		device_root.join("current_link_speed"),
		device_root.join("current_link_width"),
		device_root.join("max_link_speed"),
		device_root.join("max_link_width"),
		device_root.join("numa_node"),
	];
	let link = surface_digest("pcie-link", Some(bdf.as_bytes()), link_files.iter(), false)?;
	Ok(PciSurface {
		driver,
		firmware,
		link,
	})
}

#[cfg(test)]
mod pci_tests {
	use std::sync::atomic::{AtomicU64, Ordering};

	use super::*;

	static NONCE: AtomicU64 = AtomicU64::new(0);

	#[test]
	fn accelerator_preflight_requires_matching_vendor_and_class() {
		let root = std::env::temp_dir().join(format!(
			"recipe-native-probe-pci-{}-{}",
			std::process::id(),
			NONCE.fetch_add(1, Ordering::Relaxed)
		));
		let display = root.join("0000:01:00.0");
		let network = root.join("0000:02:00.0");
		fs::create_dir_all(&display).expect("create display fixture");
		fs::create_dir_all(&network).expect("create network fixture");
		fs::write(display.join("vendor"), b"0x1002\n").expect("write display vendor");
		fs::write(display.join("class"), b"0x030200\n").expect("write display class");
		fs::write(network.join("vendor"), b"0x1002\n").expect("write network vendor");
		fs::write(network.join("class"), b"0x020000\n").expect("write network class");

		assert!(pci_accelerator_present(&root, 0x1002).expect("scan AMD accelerator"));
		assert!(!pci_accelerator_present(&root, 0x10de).expect("scan NVIDIA accelerator"));

		fs::remove_dir_all(&root).expect("remove PCI fixture");
	}

	#[test]
	fn processing_accelerator_class_is_admitted() {
		assert!(is_accelerator_class(0x120000));
		assert!(is_accelerator_class(0x030000));
		assert!(!is_accelerator_class(0x020000));
	}
}

fn surface_digest<'a>(
	domain: &str,
	prefix: Option<&[u8]>,
	paths: impl IntoIterator<Item = &'a PathBuf>,
	require_one_file: bool,
) -> ProbeResult<String> {
	let mut digest = Sha256::new();
	digest.update(b"recipe-native-probe-surface-v1");
	digest.update(domain.as_bytes());
	if let Some(prefix) = prefix {
		digest.update((prefix.len() as u64).to_le_bytes());
		digest.update(prefix);
	}
	let mut found = 0_u64;
	for path in paths {
		let bytes = match fs::read(path) {
			Ok(bytes) => bytes,
			Err(error)
				if matches!(
					error.kind(),
					std::io::ErrorKind::NotFound | std::io::ErrorKind::PermissionDenied
				) =>
			{
				continue;
			}
			Err(error) => return Err(ProbeError::io("read PCI identity surface", path, error)),
		};
		found += 1;
		let encoded = path.as_os_str().as_encoded_bytes();
		digest.update((encoded.len() as u64).to_le_bytes());
		digest.update(encoded);
		digest.update((bytes.len() as u64).to_le_bytes());
		digest.update(bytes);
	}
	if require_one_file && found == 0 {
		return Err(ProbeError::Discovery(format!(
			"{domain} identity surface had no readable files"
		)));
	}
	digest.update(found.to_le_bytes());
	Ok(format!("{domain}:sha256={}", hex(&digest.finalize())))
}

pub(crate) fn library_identity(domain: &str, library: &PinnedLibrary) -> String {
	format!(
		"{domain}:{}:sha256={}",
		library.path.display(),
		hex(&library.digest)
	)
}

#[cfg(test)]
mod tests {
	use std::fs;
	use std::os::unix::fs::symlink;
	use std::time::{SystemTime, UNIX_EPOCH};

	use super::*;

	fn private_directory(name: &str) -> PathBuf {
		let nonce = SystemTime::now()
			.duration_since(UNIX_EPOCH)
			.expect("clock after epoch")
			.as_nanos();
		let path = std::env::temp_dir().join(format!("recipe-native-probe-{name}-{nonce}"));
		fs::create_dir(&path).expect("create test directory");
		path
	}

	#[test]
	fn selects_and_hashes_the_first_existing_explicit_library() {
		let root = private_directory("library");
		let library = root.join("libbackend.so.1");
		fs::write(&library, b"backend").expect("write fake library");
		let alias = root.join("libbackend.so");
		symlink(&library, &alias).expect("create fake soname");
		let selected = selected_library(
			&BackendLibrary {
				candidates: vec![root.join("missing"), alias],
			},
			"fake",
		)
		.expect("select library")
		.expect("library is present");
		assert_eq!(
			selected.path,
			fs::canonicalize(library).expect("canonical path")
		);
		assert_eq!(selected.digest, Sha256::digest(b"backend").as_slice());
		fs::remove_dir_all(root).expect("remove test directory");
	}

	#[test]
	fn all_missing_explicit_candidates_mean_backend_absence() {
		let root = private_directory("absent");
		let selected = selected_library(
			&BackendLibrary {
				candidates: vec![root.join("missing")],
			},
			"fake",
		)
		.expect("absence is not a backend failure");
		assert_eq!(selected, None);
		fs::remove_dir_all(root).expect("remove test directory");
	}

	#[test]
	fn relative_library_candidate_fails_closed() {
		let error = selected_library(
			&BackendLibrary {
				candidates: vec![PathBuf::from("libbackend.so")],
			},
			"fake",
		)
		.expect_err("relative library path must be rejected");
		assert!(error.to_string().contains("not absolute"));
	}

	#[test]
	fn benchmark_methodology_changes_toolchain_identity() {
		let tool = |name: &str| PinnedTool {
			path: PathBuf::from(format!("/test-tools/{name}")),
			digest: ArtifactDigest::of(name.as_bytes()),
		};
		let toolchain = OfflineToolchain {
			verifier: tool("opt"),
			llvm_codegen: tool("llc"),
			elf_linker: None,
			ptx_assembler: Some(tool("ptxas")),
		};
		let release = Label::new("test-release").expect("test label");
		let short = backend_toolchain_identity(
			&toolchain,
			&release,
			"nvidia-cuda",
			"sm_52:ptx74:dependent-f32-fma-chain-32",
		)
		.expect("short-chain identity");
		let long = backend_toolchain_identity(
			&toolchain,
			&release,
			"nvidia-cuda",
			"sm_52:ptx74:dependent-f32-fma-chain-64",
		)
		.expect("long-chain identity");
		assert_ne!(short.digest, long.digest);
	}
}
