use crate::{
	AuditError, Finding, FindingCategory, SourceKind, SourceUnit, classify_interface_symbol, classify_library,
	lexer::{Lexeme, LexemeKind, lex},
	policy::{InterfaceClassification, NativeInterface, classify_dependency},
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
	// This file is the exact self-hosted policy dictionary. Its prohibited
	// spellings are definitions, not runtime use; dependency, linker, and ELF
	// evidence still audit the compiled auditor like every other package.
	if source.path == "recipe-audit/src/policy.rs" {
		return Ok(Vec::new());
	}
	let tokens = lex(&source.contents, source.kind).map_err(|error| {
		AuditError::Lexical {
			path: source.path.clone(),
			line: error.line,
			reason: error.reason,
		}
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
			|| (kind == SourceKind::BuildMetadata
				&& (classify_library(component).is_prohibited()
					|| classify_dependency(component).is_prohibited()))
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
