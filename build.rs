use std::{env, error::Error, fs, io, path::PathBuf, process::Command};

type BuildResult<T> = Result<T, Box<dyn Error>>;

fn setting<'a>(manifest: &'a str, key: &str) -> BuildResult<&'a str> {
	let prefix = format!("{key} = ");
	manifest
		.lines()
		.find_map(|line| line.trim().strip_prefix(&prefix))
		.ok_or_else(|| io::Error::other(format!("{key} must be configured")).into())
}

fn number<'a>(manifest: &'a str, key: &str) -> BuildResult<&'a str> {
	let value = setting(manifest, key)?;
	value.parse::<f64>().map_err(|error| io::Error::other(format!("{key} must be numeric: {error}")))?;
	Ok(value)
}

fn text<'a>(manifest: &'a str, key: &str) -> BuildResult<&'a str> {
	setting(manifest, key)?
		.strip_prefix('"')
		.and_then(|value| value.strip_suffix('"'))
		.ok_or_else(|| io::Error::other(format!("{key} must be quoted")).into())
}

fn run(command: &mut Command, role: &str) -> BuildResult<()> {
	let status = command.status()?;
	if !status.success() {
		return Err(io::Error::other(format!("{role} failed")).into());
	}
	Ok(())
}

fn compile_amd(manifest: &str, out: &PathBuf) -> BuildResult<()> {
	let architecture = text(manifest, "hsa-architecture")?;
	let cpu = format!("-mcpu={architecture}");
	let output = out.join("recipe.hsaco");
	let mut command = Command::new(text(manifest, "hsa-compiler")?);
	command.args(["-target", "amdgcn-amd-amdhsa", &cpu, "-O2", "-nogpulib"]);
	for key in ["hsa-device-library", "hsa-isa-library", "hsa-finite-library", "hsa-math-library"] {
		command.args(["-Xclang", "-mlink-builtin-bitcode", "-Xclang", text(manifest, key)?]);
	}
	command.arg("amd.ll").arg("-o").arg(&output);
	run(&mut command, "HSA LLVM IR compiler")?;
	println!("cargo:rustc-env=RECIPE_HSA_CODE_OBJECT={}", output.display());
	println!("cargo:rustc-link-search=native={}", text(manifest, "hsa-library")?);
	Ok(())
}

fn compile_nvidia(manifest: &str, out: &PathBuf) -> BuildResult<()> {
	let architecture = format!("-march={}", text(manifest, "nvidia-architecture")?);
	let ptx = format!("+{}", text(manifest, "nvidia-ptx")?);
	let output = out.join("recipe.ptx");
	let mut command = Command::new(text(manifest, "nvidia-compiler")?);
	command.args([
		"-target",
		"nvptx64-nvidia-cuda",
		&architecture,
		"-Xclang",
		"-target-feature",
		"-Xclang",
		&ptx,
		"-O2",
		"-S",
		"-x",
		"ir",
		"nv.ll",
		"-Xclang",
		"-mlink-builtin-bitcode",
		"-Xclang",
		text(manifest, "nvidia-device-library")?,
		"-o",
	]);
	command.arg(&output);
	run(&mut command, "NVIDIA LLVM IR compiler")?;
	println!("cargo:rustc-env=RECIPE_NV_MODULE={}", output.display());
	println!("cargo:rustc-link-search=native={}", text(manifest, "nvidia-library")?);
	Ok(())
}

fn main() -> BuildResult<()> {
	let manifest = fs::read_to_string("Cargo.toml")?;
	for (key, environment) in [
		("epochs", "RECIPE_TRAIN_EPOCHS"),
		("learning-rate", "RECIPE_TRAIN_LEARNING_RATE"),
		("finite-difference-step", "RECIPE_TRAIN_FINITE_DIFFERENCE_STEP"),
		("kmeans-iterations", "RECIPE_KMEANS_ITERATIONS"),
		("gpu-threads", "RECIPE_GPU_THREADS"),
		("output-tolerance", "RECIPE_OUTPUT_TOLERANCE"),
		("gradient-tolerance", "RECIPE_GRADIENT_TOLERANCE"),
		("backend-tolerance", "RECIPE_BACKEND_TOLERANCE"),
	] {
		println!("cargo:rustc-env={environment}={}", number(&manifest, key)?);
	}
	let out = PathBuf::from(env::var_os("OUT_DIR").ok_or_else(|| io::Error::other("OUT_DIR must be configured"))?);
	if env::var_os("CARGO_FEATURE_AMD").is_some() {
		compile_amd(&manifest, &out)?;
	}
	if env::var_os("CARGO_FEATURE_NVIDIA").is_some() {
		compile_nvidia(&manifest, &out)?;
	}
	println!("cargo:rerun-if-changed=Cargo.toml");
	println!("cargo:rerun-if-changed=amd.ll");
	println!("cargo:rerun-if-changed=nv.ll");
	Ok(())
}
