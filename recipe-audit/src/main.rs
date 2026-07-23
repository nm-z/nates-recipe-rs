use recipe_audit::{
	AuditError, AuditInput, AuditMode, DependencyGraph, LegacyGrant, LinkerInput, collect_native_scope,
};
use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

const MAX_JSON_BYTES: u64 = 64 * 1024 * 1024;

fn main() -> ExitCode {
	match run(std::env::args_os()) {
		Ok(code) => code,
		Err(error) => {
			eprintln!("recipe-audit: {error}");
			eprintln!("{}", usage());
			ExitCode::from(2)
		}
	}
}

fn run(arguments: impl IntoIterator<Item = OsString>) -> Result<ExitCode, AuditError> {
	let parsed = Cli::parse(arguments)?;
	if parsed.help {
		println!("{}", usage());
		return Ok(ExitCode::SUCCESS);
	}

	let mode = parsed
		.mode
		.ok_or_else(|| AuditError::Configuration("--mode is required".into()))?;
	let scope = parsed
		.scope
		.ok_or_else(|| AuditError::Configuration("--scope is required".into()))?;
	let collected = collect_native_scope(&scope, &parsed.elf_paths)?;

	let dependencies = if let Some(path) = parsed.metadata {
		if parsed.package_ids.is_empty() {
			return Err(AuditError::Configuration(
				"--metadata requires at least one exact --package-id".into(),
			));
		}
		let json = read_bounded_text(&path)?;
		Some(DependencyGraph::from_cargo_metadata_json(
			&json,
			&parsed.package_ids,
		)?)
	} else {
		if !parsed.package_ids.is_empty() {
			return Err(AuditError::Configuration(
				"--package-id requires --metadata".into(),
			));
		}
		None
	};

	let legacy_grants = match (mode, parsed.legacy_grants) {
		(AuditMode::Next, None) => Vec::new(),
		(AuditMode::Next, Some(_)) => {
			return Err(AuditError::Configuration(
				"next mode cannot use --legacy-grants".into(),
			));
		}
		(AuditMode::Legacy, None) => {
			return Err(AuditError::Configuration(
				"legacy mode requires an explicit --legacy-grants JSON file".into(),
			));
		}
		(AuditMode::Legacy, Some(path)) => {
			let json = read_bounded_text(&path)?;
			serde_json::from_str::<Vec<LegacyGrant>>(&json).map_err(|error| {
				AuditError::Configuration(format!("invalid legacy grants {}: {error}", path.display()))
			})?
		}
	};

	let linker_inputs = parsed
		.linker_inputs
		.into_iter()
		.map(|argument| LinkerInput::new("<command-line>", 0, argument))
		.collect();
	let input = AuditInput {
		mode,
		sources: collected.sources,
		dependencies,
		linker_inputs,
		elf_facts: collected.elf_facts,
		legacy_grants,
	};
	let report = recipe_audit::audit(input)?;
	let output =
		serde_json::to_string_pretty(&report).map_err(|error| AuditError::Configuration(error.to_string()))?;
	println!("{output}");
	Ok(if report.passed {
		ExitCode::SUCCESS
	} else {
		ExitCode::from(1)
	})
}

fn read_bounded_text(path: &PathBuf) -> Result<String, AuditError> {
	if !path.is_absolute() {
		return Err(AuditError::Configuration(format!(
			"JSON input path must be absolute: {}",
			path.display()
		)));
	}
	let metadata = fs::metadata(path).map_err(|error| AuditError::Io {
		path: path.clone(),
		source: error,
	})?;
	if !metadata.is_file() || metadata.len() > MAX_JSON_BYTES {
		return Err(AuditError::Configuration(format!(
			"JSON input must be a regular file no larger than {MAX_JSON_BYTES} bytes: {}",
			path.display()
		)));
	}
	fs::read_to_string(path).map_err(|source| AuditError::Io {
		path: path.clone(),
		source,
	})
}

#[derive(Debug, Default)]
struct Cli {
	help: bool,
	mode: Option<AuditMode>,
	scope: Option<PathBuf>,
	metadata: Option<PathBuf>,
	package_ids: Vec<String>,
	linker_inputs: Vec<String>,
	elf_paths: Vec<PathBuf>,
	legacy_grants: Option<PathBuf>,
}

impl Cli {
	fn parse(arguments: impl IntoIterator<Item = OsString>) -> Result<Self, AuditError> {
		let mut arguments = arguments.into_iter();
		let _program = arguments.next();
		let mut cli = Self::default();
		while let Some(argument) = arguments.next() {
			let argument = argument
				.into_string()
				.map_err(|_| AuditError::Configuration("arguments must be valid UTF-8".into()))?;
			match argument.as_str() {
				"-h" | "--help" => cli.help = true,
				"--mode" => {
					reject_duplicate(cli.mode.is_some(), "--mode")?;
					cli.mode = Some(next_utf8(&mut arguments, "--mode")?.parse()?);
				}
				"--scope" => {
					reject_duplicate(cli.scope.is_some(), "--scope")?;
					cli.scope = Some(PathBuf::from(next_utf8(&mut arguments, "--scope")?));
				}
				"--metadata" => {
					reject_duplicate(cli.metadata.is_some(), "--metadata")?;
					cli.metadata = Some(PathBuf::from(next_utf8(&mut arguments, "--metadata")?));
				}
				"--package-id" => {
					cli.package_ids
						.push(next_utf8(&mut arguments, "--package-id")?);
				}
				"--link-input" => {
					cli.linker_inputs
						.push(next_utf8(&mut arguments, "--link-input")?);
				}
				"--elf" => {
					cli.elf_paths
						.push(PathBuf::from(next_utf8(&mut arguments, "--elf")?));
				}
				"--legacy-grants" => {
					reject_duplicate(cli.legacy_grants.is_some(), "--legacy-grants")?;
					cli.legacy_grants = Some(PathBuf::from(next_utf8(&mut arguments, "--legacy-grants")?));
				}
				unknown => {
					return Err(AuditError::Configuration(format!(
						"unknown argument {unknown:?}"
					)));
				}
			}
		}
		if cli.help {
			return Ok(cli);
		}
		if cli.package_ids.iter().any(String::is_empty) || cli.linker_inputs.iter().any(String::is_empty) {
			return Err(AuditError::Configuration(
				"package ids and linker inputs must be nonempty".into(),
			));
		}
		Ok(cli)
	}
}

fn next_utf8(arguments: &mut impl Iterator<Item = OsString>, option: &str) -> Result<String, AuditError> {
	arguments
		.next()
		.ok_or_else(|| AuditError::Configuration(format!("{option} requires a value")))?
		.into_string()
		.map_err(|_| AuditError::Configuration(format!("{option} value must be valid UTF-8")))
}

fn reject_duplicate(duplicate: bool, option: &str) -> Result<(), AuditError> {
	if duplicate {
		Err(AuditError::Configuration(format!(
			"{option} may be supplied only once"
		)))
	} else {
		Ok(())
	}
}

fn usage() -> &'static str {
	"usage: recipe-audit --mode next|legacy --scope ABSOLUTE_DIR\n\
     \t[--metadata ABSOLUTE_CARGO_METADATA_JSON --package-id EXACT_ID ...]\n\
     \t[--link-input EXACT_LINKER_ARGUMENT ...] [--elf ABSOLUTE_ELF ...]\n\
     \t[--legacy-grants ABSOLUTE_JSON]\n\
     legacy mode requires exact grants; next mode rejects them"
}

#[cfg(test)]
mod tests {
	use super::*;

	fn strings(values: &[&str]) -> Vec<OsString> {
		values.iter().map(OsString::from).collect()
	}

	#[test]
	fn cli_requires_explicit_values_without_touching_filesystem() {
		let cli = Cli::parse(strings(&[
			"recipe-audit",
			"--mode",
			"next",
			"--scope",
			"/fake/scope",
			"--package-id",
			"next 0.1",
			"--link-input",
			"-lcublas",
			"--elf",
			"/fake/scope/app",
		]))
		.unwrap();
		assert_eq!(cli.mode, Some(AuditMode::Next));
		assert_eq!(cli.scope, Some(PathBuf::from("/fake/scope")));
		assert_eq!(cli.linker_inputs, vec!["-lcublas"]);
		assert_eq!(cli.elf_paths, vec![PathBuf::from("/fake/scope/app")]);
	}

	#[test]
	fn cli_rejects_implicit_or_duplicate_configuration() {
		assert!(Cli::parse(strings(&["recipe-audit", "--mode"])).is_err());
		assert!(
			Cli::parse(strings(&[
				"recipe-audit",
				"--mode",
				"next",
				"--mode",
				"legacy"
			]))
			.is_err()
		);
	}
}
