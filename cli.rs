use std::{fs, path::Path, process::Command};

fn mapped(mapping: Option<&'static str>, suffix: &str) -> Vec<(String, &'static str)> { mapping.into_iter().flat_map(|values| values.split(';')).filter_map(|value| value.split_once('=')).map(|(target, path)| (format!("{target}.{suffix}"), path)).collect() }

fn export(source: &Path) {
	fs::metadata(source).unwrap_or_else(|error| panic!("cannot inspect {}: {error}", source.display()));
	let mut artifacts = mapped(option_env!("RECIPE_HSA_CODE_OBJECTS"), "hsaco");
	artifacts.extend(mapped(option_env!("RECIPE_HSA_ASSEMBLIES"), "amd.s"));
	artifacts.extend(option_env!("RECIPE_NV_PTX").map(|path| ("ptx".to_owned(), path)));
	assert!(!artifacts.is_empty(), "Recipe artifacts were not compiled");
	for (extension, compiled) in artifacts {
		let output = source.with_file_name(format!("recipe.{extension}"));
		fs::copy(compiled, &output).unwrap_or_else(|error| panic!("cannot export {}: {error}", output.display()));
		eprintln!("exported: {}", output.display());
	}
}

fn run(source: &Path) {
	let directory = std::env::current_exe().expect("cannot locate recipe").parent().expect("recipe has no parent directory").to_owned();
	let library = directory.join("librecipe.rlib");
	let dependencies = directory.join("deps");
	let output = directory.join("recipe-script");
	fs::metadata(&library).unwrap_or_else(|error| panic!("cannot inspect {}: {error}", library.display()));
	let status = Command::new("rustc").arg("--edition=2024").arg(source).arg("--extern").arg(format!("recipe={}", library.display())).arg("-L").arg(format!("dependency={}", dependencies.display())).arg("-o").arg(&output).status().expect("cannot execute rustc");
	if !status.success() {
		std::process::exit(status.code().unwrap_or(1));
	}
	let status = Command::new(output).status().expect("cannot execute Recipe script");
	std::process::exit(status.code().unwrap_or(1));
}

fn main() {
	let mut arguments = std::env::args().skip(1);
	let source = arguments.next().expect("usage: recipe <source.rs> [export]");
	let operation = arguments.next();
	assert!(arguments.next().is_none(), "usage: recipe <source.rs> [export]");
	let source = Path::new(&source);
	assert_eq!(source.extension().and_then(|value| value.to_str()), Some("rs"), "recipe requires a Rust source");
	match operation.as_deref() {
		None => run(source),
		Some("export") => export(source),
		Some(_) => panic!("usage: recipe <source.rs> [export]"),
	}
}
