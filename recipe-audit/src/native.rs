use std::{
	collections::BTreeSet,
	fs,
	path::{Path, PathBuf},
};

use goblin::elf::{Elf, section_header::SHN_UNDEF};

use crate::{ArtifactSymbol, AuditError, ElfFacts, SourceKind, SourceUnit, model::normalize_display_path};

const MAX_TEXT_FILE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_ELF_BYTES: u64 = 1024 * 1024 * 1024;
const MAX_SOURCE_FILES: usize = 100_000;

/// Filesystem facts collected from one explicit scope.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeCollection {
	pub sources: Vec<SourceUnit>,
	pub elf_facts: Vec<ElfFacts>,
}

/// Collect supported source/build files and explicitly named ELF artifacts.
///
/// The scope and every ELF path must be absolute. Symlinks are rejected and
/// ELF paths must remain under the canonical scope. `.git` and `target`
/// directories are intentionally outside source collection; artifacts to audit
/// under `target` are still supplied explicitly through `elf_paths`.
///
/// # Errors
///
/// Returns an error for I/O failures, relative or escaping paths, symlinks,
/// oversized inputs, invalid UTF-8, or malformed ELF artifacts.
pub fn collect_native_scope(scope: &Path, elf_paths: &[PathBuf]) -> Result<NativeCollection, AuditError> {
	require_absolute(scope, "scope")?;
	let scope_metadata = fs::symlink_metadata(scope).map_err(|error| AuditError::io(scope, error))?;
	if scope_metadata.file_type().is_symlink() || !scope_metadata.is_dir() {
		return Err(AuditError::Configuration(format!(
			"scope must be a real directory, not a symlink: {}",
			scope.display()
		)));
	}
	let canonical_scope = fs::canonicalize(scope).map_err(|error| AuditError::io(scope, error))?;

	let mut paths = Vec::new();
	collect_paths(&canonical_scope, &canonical_scope, &mut paths)?;
	if paths.len() > MAX_SOURCE_FILES {
		return Err(AuditError::Configuration(format!(
			"source file limit exceeded: {} > {MAX_SOURCE_FILES}",
			paths.len()
		)));
	}
	paths.sort();

	let mut sources = Vec::with_capacity(paths.len());
	for (path, kind) in paths {
		let metadata = fs::metadata(&path).map_err(|error| AuditError::io(&path, error))?;
		if metadata.len() > MAX_TEXT_FILE_BYTES {
			return Err(AuditError::Configuration(format!(
				"text file exceeds {MAX_TEXT_FILE_BYTES} bytes: {}",
				path.display()
			)));
		}
		let contents = fs::read_to_string(&path).map_err(|error| AuditError::io(&path, error))?;
		let display = relative_display(&canonical_scope, &path)?;
		sources.push(SourceUnit::new(display, kind, contents));
	}

	let mut seen_elfs = BTreeSet::new();
	let mut elf_facts = Vec::with_capacity(elf_paths.len());
	for path in elf_paths {
		require_absolute(path, "ELF path")?;
		let metadata = fs::symlink_metadata(path).map_err(|error| AuditError::io(path, error))?;
		if metadata.file_type().is_symlink() || !metadata.is_file() {
			return Err(AuditError::Configuration(format!(
				"ELF path must be a real regular file: {}",
				path.display()
			)));
		}
		let canonical = fs::canonicalize(path).map_err(|error| AuditError::io(path, error))?;
		if !canonical.starts_with(&canonical_scope) {
			return Err(AuditError::Configuration(format!(
				"ELF escapes explicit scope: {}",
				path.display()
			)));
		}
		if !seen_elfs.insert(canonical.clone()) {
			return Err(AuditError::Configuration(format!(
				"duplicate ELF path: {}",
				path.display()
			)));
		}
		let display = relative_display(&canonical_scope, &canonical)?;
		elf_facts.push(read_elf_facts_with_display(&canonical, display)?);
	}
	elf_facts.sort_by(|left, right| left.path.cmp(&right.path));

	Ok(NativeCollection { sources, elf_facts })
}

/// Extract `DT_NEEDED` entries and undefined dynamic/static symbols from ELF.
///
/// # Errors
///
/// Returns an error when `path` is not absolute, cannot be read, exceeds the
/// size bound, or is not valid ELF.
pub fn read_elf_facts(path: &Path) -> Result<ElfFacts, AuditError> {
	require_absolute(path, "ELF path")?;
	read_elf_facts_with_display(path, normalize_display_path(&path.display().to_string()))
}

fn read_elf_facts_with_display(path: &Path, display: String) -> Result<ElfFacts, AuditError> {
	let metadata = fs::metadata(path).map_err(|error| AuditError::io(path, error))?;
	if !metadata.is_file() {
		return Err(AuditError::InvalidElf {
			path: path.to_path_buf(),
			reason: "not a regular file".into(),
		});
	}
	if metadata.len() > MAX_ELF_BYTES {
		return Err(AuditError::InvalidElf {
			path: path.to_path_buf(),
			reason: format!("file exceeds {MAX_ELF_BYTES} bytes"),
		});
	}
	let bytes = fs::read(path).map_err(|error| AuditError::io(path, error))?;
	let elf = Elf::parse(&bytes).map_err(|error| {
		AuditError::InvalidElf {
			path: path.to_path_buf(),
			reason: error.to_string(),
		}
	})?;

	let mut needed = elf
		.libraries
		.iter()
		.map(|library| (*library).to_owned())
		.collect::<Vec<_>>();
	needed.sort();
	needed.dedup();

	let mut undefined = BTreeSet::new();
	for symbol in &elf.dynsyms {
		if symbol.st_shndx == SHN_UNDEF as usize
			&& symbol.st_name != 0
			&& let Some(name) = elf.dynstrtab.get_at(symbol.st_name)
			&& !name.is_empty()
		{
			undefined.insert(name.to_owned());
		}
	}
	for symbol in &elf.syms {
		if symbol.st_shndx == SHN_UNDEF as usize
			&& symbol.st_name != 0
			&& let Some(name) = elf.strtab.get_at(symbol.st_name)
			&& !name.is_empty()
		{
			undefined.insert(name.to_owned());
		}
	}

	Ok(ElfFacts::new(
		display,
		needed,
		undefined.into_iter().map(ArtifactSymbol::new).collect(),
	))
}

fn collect_paths(scope: &Path, directory: &Path, output: &mut Vec<(PathBuf, SourceKind)>) -> Result<(), AuditError> {
	let mut entries = fs::read_dir(directory)
		.map_err(|error| AuditError::io(directory, error))?
		.collect::<Result<Vec<_>, _>>()
		.map_err(|error| AuditError::io(directory, error))?;
	entries.sort_by_key(std::fs::DirEntry::file_name);

	for entry in entries {
		let path = entry.path();
		let metadata = fs::symlink_metadata(&path).map_err(|error| AuditError::io(&path, error))?;
		if metadata.file_type().is_symlink() {
			return Err(AuditError::Configuration(format!(
				"symlink inside explicit scope is not auditable: {}",
				path.display()
			)));
		}
		if metadata.is_dir() {
			let name = entry.file_name();
			if name == ".git" || name == "target" {
				continue;
			}
			collect_paths(scope, &path, output)?;
			if output.len() > MAX_SOURCE_FILES {
				return Err(AuditError::Configuration(format!(
					"source file limit exceeded under {}",
					scope.display()
				)));
			}
		} else if metadata.is_file()
			&& let Some(kind) = source_kind(&path)
		{
			output.push((path, kind));
		}
	}
	Ok(())
}

fn source_kind(path: &Path) -> Option<SourceKind> {
	let name = path.file_name()?.to_str()?;
	let extension = path.extension().and_then(std::ffi::OsStr::to_str);
	match extension {
		Some("rs") => Some(SourceKind::Rust),
		Some("zig") => Some(SourceKind::Zig),
		Some("c" | "h") => Some(SourceKind::C),
		Some("cc" | "cpp" | "cxx" | "hh" | "hpp" | "hxx") => Some(SourceKind::Cpp),
		Some("ll") => Some(SourceKind::LlvmIr),
		Some("toml" | "json" | "yaml" | "yml" | "cmake" | "mk" | "ninja" | "bazel" | "bzl") => {
			Some(SourceKind::BuildMetadata)
		}
		_ if matches!(
			name,
			"Cargo.lock"
				| "Makefile" | "CMakeLists.txt"
				| "BUILD" | "BUILD.bazel"
				| "WORKSPACE" | "WORKSPACE.bazel"
		) =>
		{
			Some(SourceKind::BuildMetadata)
		}
		_ => None,
	}
}

fn relative_display(scope: &Path, path: &Path) -> Result<String, AuditError> {
	let relative = path
		.strip_prefix(scope)
		.map_err(|_| AuditError::Configuration(format!("path escaped canonical scope: {}", path.display())))?;
	Ok(normalize_display_path(&relative.display().to_string()))
}

fn require_absolute(path: &Path, label: &str) -> Result<(), AuditError> {
	if path.is_absolute() {
		Ok(())
	} else {
		Err(AuditError::Configuration(format!(
			"{label} must be explicit and absolute: {}",
			path.display()
		)))
	}
}
