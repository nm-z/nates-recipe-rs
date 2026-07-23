use crate::lexer::{Lexeme, LexemeKind, lex};
use crate::policy::{InterfaceClassification, NativeInterface, classify_dependency};
use crate::{
	AuditError, Finding, FindingCategory, SourceKind, SourceUnit, classify_interface_symbol, classify_library,
};

/// Lexically audit one UTF-8 source or build-metadata unit.
///
/// Comments are removed by a language-aware lexer. Rules inspect complete
/// identifiers and complete string/library tokens, never arbitrary substrings.
///
/// # Errors
///
/// Returns [`AuditError::Lexical`] when the text has an unterminated comment or
/// literal, because incomplete lexical evidence cannot pass the gate.
pub fn audit_source(source: &SourceUnit) -> Result<Vec<Finding>, AuditError> {
	let tokens = lex(&source.contents, source.kind).map_err(|error| AuditError::Lexical {
		path: source.path.clone(),
		line: error.line,
		reason: error.reason,
	})?;

	let mut findings = if source.kind == SourceKind::LlvmIr {
		audit_llvm(source, &tokens)
	} else {
		Vec::new()
	};

	for (index, token) in tokens.iter().enumerate() {
		match token.kind {
			LexemeKind::Identifier => {
				if source.kind == SourceKind::LlvmIr && follows_at(&tokens, index) {
					continue;
				}
				push_interface_finding(
					&mut findings,
					source,
					token.line,
					&token.text,
					identifier_category(source.kind),
				);
			}
			LexemeKind::String => {
				if source.kind == SourceKind::LlvmIr && follows_at(&tokens, index) {
					continue;
				}
				audit_string(source, token, &mut findings);
			}
			LexemeKind::At => {}
		}
	}

	findings.sort();
	findings.dedup();
	Ok(findings)
}

fn audit_llvm(source: &SourceUnit, tokens: &[Lexeme]) -> Vec<Finding> {
	let mut findings = Vec::new();
	for (index, token) in tokens.iter().enumerate() {
		if token.kind != LexemeKind::At {
			continue;
		}
		let Some(symbol) = tokens.get(index + 1) else {
			continue;
		};
		if !matches!(symbol.kind, LexemeKind::Identifier | LexemeKind::String) {
			continue;
		}

		let same_line_prefix = tokens[..index]
			.iter()
			.rev()
			.take_while(|candidate| candidate.line == token.line);
		let mut declaration = false;
		let mut call = false;
		for candidate in same_line_prefix {
			if candidate.kind != LexemeKind::Identifier {
				continue;
			}
			match candidate.text.as_str() {
				"declare" => declaration = true,
				"call" | "invoke" | "callbr" => call = true,
				_ => {}
			}
		}

		let category = if declaration {
			Some(FindingCategory::LlvmDeclaration)
		} else if call {
			Some(FindingCategory::LlvmCall)
		} else {
			None
		};
		if let Some(category) = category {
			push_interface_finding(&mut findings, source, symbol.line, &symbol.text, category);
		}
	}
	findings
}

fn audit_string(source: &SourceUnit, token: &Lexeme, findings: &mut Vec<Finding>) {
	let trimmed = token
		.text
		.trim()
		.trim_end_matches("\\00")
		.trim_end_matches("\\0");
	let link_context = source.kind == SourceKind::BuildMetadata || line_has_link_context(source, token.line);
	let string_kind = if link_context {
		SourceKind::BuildMetadata
	} else {
		source.kind
	};
	if !string_has_prohibited_token(trimmed, string_kind) {
		return;
	}

	let category = if link_context {
		FindingCategory::BuildLinkInput
	} else if line_has_include_context(source, token.line) {
		FindingCategory::SourceApi
	} else {
		FindingCategory::RuntimeLoad
	};
	findings.push(Finding::blocking(
		category,
		source.path.clone(),
		token.line,
		trimmed,
	));
}

fn string_has_prohibited_token(value: &str, kind: SourceKind) -> bool {
	if classify_library(value).is_prohibited()
		|| (kind == SourceKind::BuildMetadata && classify_dependency(value).is_prohibited())
	{
		return true;
	}
	value.split(|character: char| {
		matches!(
			character,
			'/' | '\\' | '=' | ',' | ':' | '(' | ')' | '[' | ']' | ';'
		) || character.is_ascii_whitespace()
	})
	.filter(|component| !component.is_empty())
	.any(|component| {
		classify_interface_symbol(component).is_prohibited()
			|| classify_library(component).is_prohibited()
			|| (kind == SourceKind::BuildMetadata && classify_dependency(component).is_prohibited())
	})
}

fn push_interface_finding(
	findings: &mut Vec<Finding>,
	source: &SourceUnit,
	line: u64,
	symbol: &str,
	normal_category: FindingCategory,
) {
	let mut classification = classify_interface_symbol(symbol);
	if classification == InterfaceClassification::Unknown && source.kind == SourceKind::BuildMetadata {
		classification = classify_dependency(symbol);
	}
	let category = match classification {
		InterfaceClassification::Prohibited(
			NativeInterface::CudaDriverOutsideAllowlist | NativeInterface::DirectKfd,
		) => FindingCategory::DisallowedNativeInterface,
		InterfaceClassification::Prohibited(_) => {
			if source.kind == SourceKind::BuildMetadata {
				FindingCategory::BuildLinkInput
			} else {
				normal_category
			}
		}
		InterfaceClassification::Allowed(_) | InterfaceClassification::Unknown => return,
	};
	findings.push(Finding::blocking(
		category,
		source.path.clone(),
		line,
		symbol,
	));
}

fn identifier_category(kind: SourceKind) -> FindingCategory {
	if kind == SourceKind::BuildMetadata {
		FindingCategory::BuildLinkInput
	} else {
		FindingCategory::SourceApi
	}
}

fn follows_at(tokens: &[Lexeme], index: usize) -> bool {
	index > 0 && tokens[index - 1].kind == LexemeKind::At && tokens[index - 1].line == tokens[index].line
}

fn line_has_link_context(source: &SourceUnit, line: u64) -> bool {
	line_text(source, line).is_some_and(|text| {
		[
			"rustc-link-lib",
			"rustc-link-arg",
			"#[link",
			"target_link_libraries",
			"linkSystemLibrary",
			"-l",
		]
		.iter()
		.any(|marker| text.contains(marker))
	})
}

fn line_has_include_context(source: &SourceUnit, line: u64) -> bool {
	line_text(source, line).is_some_and(|text| {
		["#include", "@import", "@cImport"]
			.iter()
			.any(|marker| text.contains(marker))
	})
}

fn line_text(source: &SourceUnit, line: u64) -> Option<&str> {
	let line_index = line
		.checked_sub(1)
		.and_then(|line| usize::try_from(line).ok())?;
	source.contents.lines().nth(line_index)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn source_scanner_uses_lexical_tokens() {
		let source = SourceUnit::new(
			"src/backend.rs",
			SourceKind::Rust,
			r"
                let architecture = relationship(current);
                // hipMalloc(ptr, 4);
                hsa_queue_create(agent, 8, ty, cb, data, 0, 0, &queue);
                cuLaunchKernel(function);
                cudaMalloc(pointer, 4);
            ",
		);
		let findings = audit_source(&source).unwrap();
		assert_eq!(findings.len(), 1);
		assert_eq!(findings[0].symbol, "cudaMalloc");
		assert_eq!(findings[0].category, FindingCategory::SourceApi);
	}

	#[test]
	fn llvm_declarations_and_calls_have_distinct_categories() {
		let source = SourceUnit::new(
			"kernel.ll",
			SourceKind::LlvmIr,
			"declare i32 @rocblas_sgemm(ptr)\ndefine void @owned() {\n  %x = call i32 @cublasCreate_v2(ptr null)\n  ret void\n}\n",
		);
		let findings = audit_source(&source).unwrap();
		assert!(findings.iter().any(|finding| {
			finding.category == FindingCategory::LlvmDeclaration && finding.symbol == "rocblas_sgemm"
		}));
		assert!(findings.iter().any(|finding| {
			finding.category == FindingCategory::LlvmCall && finding.symbol == "cublasCreate_v2"
		}));
	}

	#[test]
	fn runtime_library_strings_are_exact() {
		let source = SourceUnit::new(
			"src/load.rs",
			SourceKind::Rust,
			r#"
                Library::new("libcublas.so.12");
                Library::new("librelationship.so");
            "#,
		);
		let findings = audit_source(&source).unwrap();
		assert_eq!(findings.len(), 1);
		assert_eq!(findings[0].category, FindingCategory::RuntimeLoad);
		assert_eq!(findings[0].symbol, "libcublas.so.12");
	}

	#[test]
	fn diagnostic_strings_preserve_allowed_driver_symbols() {
		let source = SourceUnit::new(
			"src/driver.rs",
			SourceKind::Rust,
			r#"
				check("cuCtxPopCurrent_v2(after create)");
				check("cuDeviceGetAttribute(COMPUTE_CAPABILITY_MAJOR)");
				check("cuDeviceGetPCIBusId");
			"#,
		);
		assert!(audit_source(&source).unwrap().is_empty());
	}

	#[test]
	fn quoted_hip_headers_and_build_dependencies_are_caught() {
		let header = SourceUnit::new("shim.c", SourceKind::C, "#include \"hip/hip_runtime.h\"\n");
		assert_eq!(
			audit_source(&header).unwrap()[0].category,
			FindingCategory::SourceApi
		);

		let manifest = SourceUnit::new(
			"Cargo.toml",
			SourceKind::BuildMetadata,
			"[dependencies]\nhip-sys = \"1\"\n",
		);
		let findings = audit_source(&manifest).unwrap();
		assert!(findings.iter().any(|finding| finding.symbol == "hip-sys"));

		let build_script = SourceUnit::new(
			"build.rs",
			SourceKind::Rust,
			"fn main() { println!(\"cargo:rustc-link-lib=static=rocblas\"); }\n",
		);
		let findings = audit_source(&build_script).unwrap();
		assert_eq!(findings[0].category, FindingCategory::BuildLinkInput);
	}
}
