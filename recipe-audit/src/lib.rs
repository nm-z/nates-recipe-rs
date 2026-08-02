//! Deterministic audit gate for Recipe's native-interface boundary.
//!
//! The crate deliberately separates collection from policy evaluation. Tests
//! and build systems can inject dependency, linker, and ELF facts without
//! consulting the host. The CLI is the opt-in native collector and requires an
//! explicit scope.

mod artifact;
mod dependency;
mod error;
mod lexer;
mod model;
mod native;
mod policy;
mod source;

use std::collections::{BTreeMap, BTreeSet};

pub use artifact::{audit_elf_facts, audit_linker_inputs};
pub use dependency::{DependencyEdge, DependencyGraph, DependencyPackage};
pub use error::AuditError;
pub use model::{
	ArtifactSymbol, AuditInput, AuditMode, AuditReport, ElfFacts, Finding, FindingCategory, FindingDisposition,
	LegacyGrant, LinkerInput, SourceKind, SourceUnit,
};
pub use native::{NativeCollection, collect_native_scope, read_elf_facts};
pub use policy::{
	CUDA_DRIVER_API_ALLOWLIST, InterfaceClassification, NativeInterface, classify_interface_symbol, classify_library,
};
pub use source::audit_source;

/// Evaluate all injected facts and return a deterministic report.
///
/// Invalid graphs, duplicate facts, malformed grants, and stale legacy grants
/// are errors rather than clean reports. This keeps absent evidence from being
/// interpreted as proof.
///
/// # Errors
///
/// Returns [`AuditError`] when any supplied fact is malformed, incomplete, or
/// inconsistent with the selected mode.
pub fn audit(input: AuditInput) -> Result<AuditReport, AuditError> {
	input.validate()?;
	let AuditInput {
		mode,
		sources,
		dependencies,
		linker_inputs,
		elf_facts,
		legacy_grants,
	} = input;

	let mut findings = Vec::new();
	for source in &sources {
		findings.extend(audit_source(source)?);
	}
	if let Some(graph) = &dependencies {
		findings.extend(graph.audit()?);
	}
	findings.extend(audit_linker_inputs(&linker_inputs));
	for elf in &elf_facts {
		findings.extend(audit_elf_facts(elf));
	}

	findings.sort();
	findings.dedup();

	match mode {
		AuditMode::Next => {
			if !legacy_grants.is_empty() {
				return Err(AuditError::Configuration(
					"next mode cannot accept legacy grants".into(),
				));
			}
		}
		AuditMode::Legacy => apply_legacy_grants(&mut findings, &legacy_grants)?,
	}

	Ok(AuditReport::new(mode, findings))
}

fn apply_legacy_grants(findings: &mut [Finding], grants: &[LegacyGrant]) -> Result<(), AuditError> {
	let mut indexed = BTreeMap::new();
	for grant in grants {
		grant.validate()?;
		if indexed.insert(grant.key(), grant).is_some() {
			return Err(AuditError::Configuration(format!(
				"duplicate legacy grant: {}:{}:{}:{}",
				grant.category.as_str(),
				grant.path,
				grant.line,
				grant.symbol
			)));
		}
	}

	let mut used = BTreeSet::new();
	for finding in findings {
		if indexed.contains_key(&finding.key()) {
			finding.disposition = FindingDisposition::Grandfathered;
			used.insert(finding.key());
		}
	}

	let unused = indexed
		.keys()
		.filter(|key| !used.contains(*key))
		.map(|key| format!("{}:{}:{}:{}", key.0.as_str(), key.1, key.2, key.3))
		.collect::<Vec<_>>();
	if unused.is_empty() {
		Ok(())
	} else {
		Err(AuditError::Configuration(format!(
			"unused or stale legacy grants: {}",
			unused.join(", ")
		)))
	}
}
