#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum AuditKind { ExternalIrDeclaration, }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditFinding { pub kind: AuditKind, pub line: usize, pub token: String, }

/// Reject external LLVM declarations except target intrinsics.
///
/// Recipe-generated kernels are closed modules: a host runtime or operation
/// library cannot be smuggled in through an unresolved call. The broader
/// repository, dependency, linker, dynamic-load, and final-ELF boundary is
/// defined in `CONTRIBUTING.md`.
#[must_use]
pub fn audit_llvm_ir(ir: &str) -> Vec<AuditFinding> { let mut findings = Vec::new();
	for (line_index, line) in ir.lines().enumerate() { let trimmed = line.trim_start();
		if !trimmed.starts_with("declare ") {
			continue; }
		let Some(at) = trimmed.find('@') else {
			continue; }; let symbol = &trimmed[at + 1..]; let symbol = symbol
			.split(|character: char| !(character.is_ascii_alphanumeric() || matches!(character, '.' | '_')))
			.next() .unwrap_or_default();
		if !symbol.starts_with("llvm.") {
			findings.push(AuditFinding { kind: AuditKind::ExternalIrDeclaration, line: line_index + 1, token: symbol.to_owned(),
			}); } }
	findings }
