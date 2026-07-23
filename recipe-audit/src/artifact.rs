use crate::policy::classify_interface_symbol;
use crate::{ElfFacts, Finding, FindingCategory, InterfaceClassification, LinkerInput, classify_library};

/// Audit declarative and command-line linker inputs.
#[must_use]
pub fn audit_linker_inputs(inputs: &[LinkerInput]) -> Vec<Finding> {
	let mut findings = Vec::new();
	for input in inputs {
		for candidate in linker_candidates(&input.argument) {
			if classify_library(candidate).is_prohibited() || classify_interface_symbol(candidate).is_prohibited()
			{
				findings.push(Finding::blocking(
					FindingCategory::BuildLinkInput,
					input.path.clone(),
					input.line,
					candidate,
				));
			}
		}
	}
	findings.sort();
	findings.dedup();
	findings
}

/// Audit facts extracted from one ELF or supplied by an artifact producer.
#[must_use]
pub fn audit_elf_facts(elf: &ElfFacts) -> Vec<Finding> {
	let mut findings = elf
		.needed
		.iter()
		.filter(|library| classify_library(library).is_prohibited())
		.map(|library| {
			Finding::blocking(
				FindingCategory::DynamicNeeded,
				elf.path.clone(),
				0,
				library.clone(),
			)
		})
		.collect::<Vec<_>>();

	findings.extend(
		elf.undefined_symbols
			.iter()
			.filter(|symbol| {
				matches!(
					classify_interface_symbol(&symbol.name),
					InterfaceClassification::Prohibited(_)
				)
			})
			.map(|symbol| {
				Finding::blocking(
					FindingCategory::UndefinedSymbol,
					elf.path.clone(),
					0,
					symbol.name.clone(),
				)
			}),
	);
	findings.sort();
	findings.dedup();
	findings
}

fn linker_candidates(argument: &str) -> Vec<&str> {
	let mut candidates = vec![argument.trim()];
	candidates.extend(
		argument
			.split(|character: char| {
				character.is_ascii_whitespace()
					|| matches!(character, ',' | '=' | ':' | '(' | ')' | '[' | ']' | ';')
			})
			.map(|candidate| candidate.trim_matches(|character| matches!(character, '"' | '\'' | '{' | '}')))
			.filter(|candidate| !candidate.is_empty()),
	);
	candidates.sort_unstable();
	candidates.dedup();
	candidates
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::ArtifactSymbol;

	#[test]
	fn static_link_inputs_are_caught_without_substring_matches() {
		let findings = audit_linker_inputs(&[
			LinkerInput::new("build.rs", 9, "-Wl,-Bstatic,-lrocblas,-Bdynamic"),
			LinkerInput::new("build.rs", 10, "/opt/cuda/lib64/libcublas_static.a"),
			LinkerInput::new("build.rs", 11, "-lrelationship"),
		]);
		assert_eq!(findings.len(), 2);
		assert!(findings.iter().any(|finding| finding.symbol == "-lrocblas"));
		assert!(
			findings
				.iter()
				.any(|finding| finding.symbol == "/opt/cuda/lib64/libcublas_static.a")
		);
	}

	#[test]
	fn dynamic_needed_and_undefined_symbols_are_distinct() {
		let facts = ElfFacts::new(
			"target/app",
			vec!["libcudart.so.11".into(), "libcuda.so.1".into()],
			vec![
				ArtifactSymbol::new("cudnnCreate"),
				ArtifactSymbol::new("cuLaunchKernel"),
				ArtifactSymbol::new("relationship"),
			],
		);
		let findings = audit_elf_facts(&facts);
		assert_eq!(findings.len(), 2);
		assert!(findings.iter().any(|finding| {
			finding.category == FindingCategory::DynamicNeeded && finding.symbol == "libcudart.so.11"
		}));
		assert!(findings.iter().any(|finding| {
			finding.category == FindingCategory::UndefinedSymbol && finding.symbol == "cudnnCreate"
		}));
	}
}
