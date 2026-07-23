use recipe_audit::{
	ArtifactSymbol, AuditInput, AuditMode, DependencyGraph, ElfFacts, FindingCategory, FindingDisposition,
	LegacyGrant, LinkerInput, SourceKind, SourceUnit, audit,
};

fn complete_fixture() -> AuditInput {
	let root = "next 0.1.0 (path+file:///fake/next)".to_owned();
	AuditInput {
		mode: AuditMode::Next,
		sources: vec![
			SourceUnit::new(
				"src/prohibited.rs",
				SourceKind::Rust,
				include_str!("fixtures/prohibited.rs"),
			),
			SourceUnit::new(
				"kernels/prohibited.ll",
				SourceKind::LlvmIr,
				include_str!("fixtures/prohibited.ll"),
			),
			SourceUnit::new(
				"src/clean.rs",
				SourceKind::Rust,
				include_str!("fixtures/clean.rs"),
			),
		],
		dependencies: Some(DependencyGraph::from_cargo_metadata_json(
			include_str!("fixtures/metadata.json"),
			&[root],
		)
		.unwrap()),
		linker_inputs: vec![
			LinkerInput::new("build/link.rsp", 3, "-Wl,-Bstatic,-lrocfft,-Bdynamic"),
			LinkerInput::new("build/link.rsp", 4, "-larchitecture"),
		],
		elf_facts: vec![ElfFacts::new(
			"artifacts/backend.so",
			vec!["libcudart.so.12".into(), "libhsa-runtime64.so.1".into()],
			vec![
				ArtifactSymbol::new("miopenCreate"),
				ArtifactSymbol::new("cuLaunchKernel"),
				ArtifactSymbol::new("relationship"),
			],
		)],
		legacy_grants: Vec::new(),
	}
}

#[test]
fn fake_fixture_catches_every_required_evidence_class() {
	let report = audit(complete_fixture()).unwrap();
	assert!(!report.passed);
	for category in [
		FindingCategory::SourceApi,
		FindingCategory::RuntimeLoad,
		FindingCategory::LlvmDeclaration,
		FindingCategory::LlvmCall,
		FindingCategory::Dependency,
		FindingCategory::BuildLinkInput,
		FindingCategory::DynamicNeeded,
		FindingCategory::UndefinedSymbol,
	] {
		assert!(
			report.findings
				.iter()
				.any(|finding| finding.category == category),
			"missing {category}"
		);
	}
}

#[test]
fn ordinary_words_and_allowed_native_interfaces_do_not_match() {
	let report = audit(AuditInput {
		mode: AuditMode::Next,
		sources: vec![SourceUnit::new(
			"src/clean.rs",
			SourceKind::Rust,
			include_str!("fixtures/clean.rs"),
		)],
		dependencies: None,
		linker_inputs: vec![LinkerInput::new(
			"build/link.rsp",
			1,
			"-lrelationship -larchitecture -lcuda -lhsa-runtime64",
		)],
		elf_facts: vec![ElfFacts::new(
			"artifacts/clean.so",
			vec!["libcuda.so.1".into(), "libhsa-runtime64.so.1".into()],
			vec![
				ArtifactSymbol::new("cuLaunchKernel"),
				ArtifactSymbol::new("hsa_queue_create"),
			],
		)],
		legacy_grants: Vec::new(),
	})
	.unwrap();
	assert!(report.passed, "{:?}", report.findings);
	assert!(report.findings.is_empty());
}

#[test]
fn legacy_grants_are_exact_and_stale_grants_fail_closed() {
	let mut input = AuditInput::empty(AuditMode::Legacy);
	input.sources.push(SourceUnit::new(
		"src/legacy.rs",
		SourceKind::Rust,
		"fn run() {\n    hipMalloc(ptr, 4);\n}\n",
	));
	input.legacy_grants.push(LegacyGrant::new(
		FindingCategory::SourceApi,
		"src/legacy.rs",
		2,
		"hipMalloc",
	));

	let report = audit(input.clone()).unwrap();
	assert!(report.passed);
	assert_eq!(
		report.findings[0].disposition,
		FindingDisposition::Grandfathered
	);

	input.legacy_grants[0].line = 3;
	assert!(audit(input).is_err());
}

#[test]
fn next_mode_rejects_even_exact_legacy_grants() {
	let mut input = AuditInput::empty(AuditMode::Next);
	input.legacy_grants.push(LegacyGrant::new(
		FindingCategory::SourceApi,
		"src/legacy.rs",
		1,
		"hipMalloc",
	));
	assert!(audit(input).is_err());
}

#[test]
fn findings_are_deterministic_and_include_required_coordinates() {
	let first = audit(complete_fixture()).unwrap();
	let second = audit(complete_fixture()).unwrap();
	assert_eq!(first, second);
	assert!(first.findings.iter().all(|finding| {
		!finding.path.is_empty() && !finding.symbol.is_empty() && !finding.category.as_str().is_empty()
	}));
}

#[test]
fn malformed_lexical_input_fails_closed() {
	let mut input = AuditInput::empty(AuditMode::Next);
	input.sources.push(SourceUnit::new(
		"src/truncated.rs",
		SourceKind::Rust,
		"/* incomplete",
	));
	assert!(audit(input).is_err());
}
