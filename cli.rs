use std::{fs, os::unix::process::CommandExt, path::Path, path::PathBuf, process::Command};

const USAGE: &str = "usage: recipe [--device <name>] <source.rs> [export] | recipe --runtime | recipe --probe";

fn invalid(message: &str) -> ! {
	eprintln!("{message}");
	std::process::exit(2)
}

fn mapped(mapping: Option<&'static str>, suffix: &str) -> Vec<(String, &'static str)> { mapping.into_iter().flat_map(|values| values.split(';')).filter_map(|value| value.split_once('=')).map(|(target, path)| (format!("{target}.{suffix}"), path)).collect() }

fn export(source: &Path, selected: Option<&str>) {
	fs::metadata(source).unwrap_or_else(|error| panic!("cannot inspect {}: {error}", source.display())); let device = selected.map(|name| name.rsplit(':').next().unwrap_or(name));
	if device.is_some_and(|name| name != "cpu" && !name.starts_with("amd") && !name.starts_with("nv")) { invalid("export device must be cpu, an amd device, or an nv device") }
	let mut artifacts = mapped(option_env!("RECIPE_HSA_CODE_OBJECTS"), "hsaco"); artifacts.push(("cpu.a".to_owned(), concat!(env!("OUT_DIR"), "/librecipe_cpu.a")));
	artifacts.extend(mapped(option_env!("RECIPE_HSA_ASSEMBLIES"), "amd.s")); artifacts.extend(option_env!("RECIPE_NV_PTX").map(|path| ("ptx".to_owned(), path)));
	artifacts.retain(|(extension, _)| device.is_none_or(|name| match name { "cpu" => extension == "cpu.a", name if name.starts_with("amd") => extension.ends_with(".hsaco") || extension.ends_with(".amd.s"), _ => extension == "ptx" }));
	assert!(!artifacts.is_empty(), "Recipe artifacts for {} were not compiled", selected.unwrap_or("this build"));
	for (extension, compiled) in artifacts {
		let output = source.with_file_name(format!("recipe.{extension}"));
		fs::copy(compiled, &output).unwrap_or_else(|error| panic!("cannot export {}: {error}", output.display()));
		eprintln!("exported: {}", output.display());
	}
}

/// The newest Recipe library beside this binary, with the direct one winning ties.
fn library_path(directory: &Path) -> PathBuf {
	let named = |path: &Path| path.file_name().map(|value| value.to_string_lossy().into_owned()).unwrap_or_default();
	let direct = directory.join("librecipe.rlib");
	fs::read_dir(directory.join("deps")).into_iter().flatten().flatten().map(|entry| entry.path()).filter(|path| named(path).starts_with("librecipe-") && named(path).ends_with(".rlib")).chain([direct.clone()]).max_by_key(|path| path.metadata().and_then(|metadata| metadata.modified()).ok()).unwrap_or(direct)
}

fn run(source: &Path, device: Option<&str>) {
	// The public boundary submits every declaration to the authoritative runtime;
	// only an executor the runtime dispatched, marked by the RECIPE_JOB it was
	// granted, compiles and becomes one here.
	if std::env::var_os("RECIPE_JOB").is_none() {
		recipe::submit(source, device)
	}
	let binary = std::env::current_exe().expect("cannot locate recipe");
	let directory = binary.parent().expect("recipe has no parent directory").to_owned();
	let library = library_path(&directory);
	// The runtime staged this declaration and removes it and this executable when
	// the job reaches a terminal state, so neither outlives the job.
	let output = source.with_extension("job");
	let status = Command::new("rustc").arg("--edition=2024").arg(source).arg("--extern").arg(format!("recipe={}", library.display())).arg("-L").arg(format!("dependency={}", directory.join("deps").display())).arg("-o").arg(&output).status().expect("cannot execute rustc");
	if !status.success() {
		fs::remove_file(&output).ok();
		std::process::exit(status.code().unwrap_or(1));
	}
	// The executor becomes the declaration, so one process holds the granted
	// device, takes the runtime's interrupt directly, and reports its own exit.
	let mut command = Command::new(&output);
	command.env("RECIPE_BIN", &binary);
	if let Some(device) = device {
		command.env("RECIPE_DEVICE", device);
	}
	panic!("cannot execute Recipe script: {}", command.exec());
}

fn main() {
	let mut arguments = std::env::args().skip(1);
	let (mut source, mut operation, mut device) = (None::<String>, None::<String>, None::<String>);
	while let Some(argument) = arguments.next() {
		if argument == "--runtime" {
			recipe::runtime()
		}
		if argument == "--probe" {
			recipe::probe()
		}
		if argument == "--device" {
			let selected = arguments.next().unwrap_or_else(|| invalid(USAGE));
			if device.is_some() {
				invalid("duplicate --device")
			}
			device = Some(selected);
			continue;
		}
		if argument.starts_with("--") {
			invalid(USAGE)
		}
		if source.is_none() {
			source = Some(argument);
			continue;
		}
		if operation.is_none() {
			operation = Some(argument);
			continue;
		}
		invalid(USAGE)
	}
	let source = source.unwrap_or_else(|| invalid(USAGE));
	let device = device.as_deref();
	let source = Path::new(&source);
	if source.extension().and_then(|value| value.to_str()) != Some("rs") {
		invalid("recipe requires a Rust source")
	}
	match operation.as_deref() {
		None => run(source, device),
		Some("export") => export(source, device),
		Some(_) => invalid(USAGE),
	}
}
