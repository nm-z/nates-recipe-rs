use crate::AuditError;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Which product boundary is being audited.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AuditMode {
	/// Production replacement code. No exceptions are accepted.
	Next,
	/// Legacy code. Every exception requires an exact typed grant.
	Legacy,
}

impl AuditMode {
	#[must_use]
	pub const fn as_str(self) -> &'static str {
		match self {
			Self::Next => "next",
			Self::Legacy => "legacy",
		}
	}
}

impl std::str::FromStr for AuditMode {
	type Err = AuditError;

	fn from_str(value: &str) -> Result<Self, Self::Err> {
		match value {
			"next" => Ok(Self::Next),
			"legacy" => Ok(Self::Legacy),
			_ => Err(AuditError::Configuration(format!(
				"unknown mode {value:?}; expected next or legacy"
			))),
		}
	}
}

impl fmt::Display for AuditMode {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter.write_str(self.as_str())
	}
}

/// Stable policy category for a finding or legacy grant.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FindingCategory {
	Dependency,
	SourceApi,
	BuildLinkInput,
	DynamicNeeded,
	LlvmDeclaration,
	LlvmCall,
	UndefinedSymbol,
	RuntimeLoad,
	DisallowedNativeInterface,
}

impl FindingCategory {
	#[must_use]
	pub const fn as_str(self) -> &'static str {
		match self {
			Self::Dependency => "dependency",
			Self::SourceApi => "source-api",
			Self::BuildLinkInput => "build-link-input",
			Self::DynamicNeeded => "dynamic-needed",
			Self::LlvmDeclaration => "llvm-declaration",
			Self::LlvmCall => "llvm-call",
			Self::UndefinedSymbol => "undefined-symbol",
			Self::RuntimeLoad => "runtime-load",
			Self::DisallowedNativeInterface => "disallowed-native-interface",
		}
	}
}

impl fmt::Display for FindingCategory {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter.write_str(self.as_str())
	}
}

/// Whether a finding blocks the selected mode.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FindingDisposition {
	Blocking,
	Grandfathered,
}

/// One exact, stable audit result.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Finding {
	pub category: FindingCategory,
	/// Slash-normalized path supplied by the explicit scope.
	pub path: String,
	/// One-based text line, or zero for binary/graph facts.
	pub line: u64,
	/// Exact token, package, library, or artifact symbol that triggered policy.
	pub symbol: String,
	pub disposition: FindingDisposition,
}

impl Finding {
	#[must_use]
	pub fn blocking(
		category: FindingCategory,
		path: impl Into<String>,
		line: u64,
		symbol: impl Into<String>,
	) -> Self {
		Self {
			category,
			path: normalize_display_path(&path.into()),
			line,
			symbol: symbol.into(),
			disposition: FindingDisposition::Blocking,
		}
	}

	pub(crate) fn key(&self) -> (FindingCategory, String, u64, String) {
		(
			self.category,
			self.path.clone(),
			self.line,
			self.symbol.clone(),
		)
	}
}

/// Exact exception for one known legacy finding.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LegacyGrant {
	pub category: FindingCategory,
	pub path: String,
	pub line: u64,
	pub symbol: String,
}

impl LegacyGrant {
	#[must_use]
	pub fn new(category: FindingCategory, path: impl Into<String>, line: u64, symbol: impl Into<String>) -> Self {
		Self {
			category,
			path: normalize_display_path(&path.into()),
			line,
			symbol: symbol.into(),
		}
	}

	pub(crate) fn validate(&self) -> Result<(), AuditError> {
		if self.path.is_empty() || self.symbol.is_empty() {
			return Err(AuditError::Configuration(
				"legacy grants require nonempty exact path and symbol".into(),
			));
		}
		if self.path.contains('*') || self.path.contains('?') || self.symbol.contains('*') {
			return Err(AuditError::Configuration(
				"legacy grants cannot contain glob or wildcard syntax".into(),
			));
		}
		if normalize_display_path(&self.path) != self.path {
			return Err(AuditError::Configuration(format!(
				"legacy grant path is not normalized: {:?}",
				self.path
			)));
		}
		Ok(())
	}

	pub(crate) fn key(&self) -> (FindingCategory, String, u64, String) {
		(
			self.category,
			self.path.clone(),
			self.line,
			self.symbol.clone(),
		)
	}
}

/// Language-specific lexical behavior for an injected source.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum SourceKind {
	Rust,
	Zig,
	C,
	Cpp,
	LlvmIr,
	BuildMetadata,
}

/// A UTF-8 source or build file in the explicit audit scope.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceUnit {
	pub path: String,
	pub kind: SourceKind,
	pub contents: String,
}

impl SourceUnit {
	#[must_use]
	pub fn new(path: impl Into<String>, kind: SourceKind, contents: impl Into<String>) -> Self {
		Self {
			path: normalize_display_path(&path.into()),
			kind,
			contents: contents.into(),
		}
	}
}

/// One linker argument or declarative linker input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LinkerInput {
	pub path: String,
	pub line: u64,
	pub argument: String,
}

impl LinkerInput {
	#[must_use]
	pub fn new(path: impl Into<String>, line: u64, argument: impl Into<String>) -> Self {
		Self {
			path: normalize_display_path(&path.into()),
			line,
			argument: argument.into(),
		}
	}
}

/// A symbol extracted from a native artifact.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ArtifactSymbol {
	pub name: String,
}

impl ArtifactSymbol {
	#[must_use]
	pub fn new(name: impl Into<String>) -> Self {
		Self { name: name.into() }
	}
}

/// Relevant facts from one ELF artifact.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ElfFacts {
	pub path: String,
	pub needed: Vec<String>,
	pub undefined_symbols: Vec<ArtifactSymbol>,
}

impl ElfFacts {
	#[must_use]
	pub fn new(path: impl Into<String>, needed: Vec<String>, undefined_symbols: Vec<ArtifactSymbol>) -> Self {
		Self {
			path: normalize_display_path(&path.into()),
			needed,
			undefined_symbols,
		}
	}
}

/// Complete injected audit input.
#[derive(Clone, Debug)]
pub struct AuditInput {
	pub mode: AuditMode,
	pub sources: Vec<SourceUnit>,
	pub dependencies: Option<crate::DependencyGraph>,
	pub linker_inputs: Vec<LinkerInput>,
	pub elf_facts: Vec<ElfFacts>,
	pub legacy_grants: Vec<LegacyGrant>,
}

impl AuditInput {
	#[must_use]
	pub const fn empty(mode: AuditMode) -> Self {
		Self {
			mode,
			sources: Vec::new(),
			dependencies: None,
			linker_inputs: Vec::new(),
			elf_facts: Vec::new(),
			legacy_grants: Vec::new(),
		}
	}

	pub(crate) fn validate(&self) -> Result<(), AuditError> {
		validate_unique_paths(&self.sources, |source| source.path.as_str(), "source")?;
		validate_unique_paths(&self.elf_facts, |elf| elf.path.as_str(), "ELF")?;
		for source in &self.sources {
			validate_display_path(&source.path, "source")?;
		}
		for input in &self.linker_inputs {
			validate_display_path(&input.path, "linker-input")?;
			if input.argument.is_empty() {
				return Err(AuditError::Configuration(format!(
					"empty linker input at {}:{}",
					input.path, input.line
				)));
			}
		}
		for elf in &self.elf_facts {
			validate_display_path(&elf.path, "ELF")?;
			if elf.needed.iter().any(String::is_empty)
				|| elf.undefined_symbols
					.iter()
					.any(|symbol| symbol.name.is_empty())
			{
				return Err(AuditError::Configuration(format!(
					"ELF facts contain an empty library or symbol: {}",
					elf.path
				)));
			}
		}
		Ok(())
	}
}

fn validate_unique_paths<T, F>(values: &[T], path: F, label: &str) -> Result<(), AuditError>
where
	F: Fn(&T) -> &str,
{
	let mut seen = std::collections::BTreeSet::new();
	for value in values {
		let current = path(value);
		if !seen.insert(current) {
			return Err(AuditError::Configuration(format!(
				"duplicate {label} path: {current}"
			)));
		}
	}
	Ok(())
}

fn validate_display_path(path: &str, label: &str) -> Result<(), AuditError> {
	if path.is_empty() || normalize_display_path(path) != path {
		Err(AuditError::Configuration(format!(
			"{label} path is empty or not normalized: {path:?}"
		)))
	} else {
		Ok(())
	}
}

/// Deterministic result. A report passes only when no finding remains blocking.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AuditReport {
	pub mode: AuditMode,
	pub passed: bool,
	pub findings: Vec<Finding>,
}

impl AuditReport {
	pub(crate) fn new(mode: AuditMode, findings: Vec<Finding>) -> Self {
		let passed = findings
			.iter()
			.all(|finding| finding.disposition == FindingDisposition::Grandfathered);
		Self {
			mode,
			passed,
			findings,
		}
	}

	#[must_use]
	pub fn blocking_count(&self) -> usize {
		self.findings
			.iter()
			.filter(|finding| finding.disposition == FindingDisposition::Blocking)
			.count()
	}
}

pub(crate) fn normalize_display_path(path: &str) -> String {
	path.replace('\\', "/")
}
