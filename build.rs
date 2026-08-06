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
	command.arg("amd-nv.ll").arg("-o").arg(&output);
	run(&mut command, "HSA LLVM IR compiler")?;
	println!("cargo:rustc-env=RECIPE_HSA_CODE_OBJECT={}", output.display());
	println!("cargo:rustc-link-search=native={}", text(manifest, "hsa-library")?);
	Ok(())
}

fn compile_nvidia(manifest: &str, out: &PathBuf) -> BuildResult<()> {
	let gpu_architecture = text(manifest, "nvidia-architecture")?;
	let architecture = format!("-march={gpu_architecture}");
	let assembler_architecture = format!("-arch={gpu_architecture}");
	let ptx = format!("+{}", text(manifest, "nvidia-ptx")?);
	let ptx_output = out.join("recipe.ptx");
	let output = out.join("recipe.cubin");
	let source = out.join("recipe-nvidia.ll");
	let ir = fs::read_to_string("amd-nv.ll")?
		.replace("amdgcn-amd-amdhsa", "nvptx64-nvidia-cuda")
		.replace("llvm.amdgcn.workitem.id.x", "llvm.nvvm.read.ptx.sreg.tid.x")
		.replace("llvm.amdgcn.workgroup.id.x", "llvm.nvvm.read.ptx.sreg.ctaid.x")
		.replace("llvm.amdgcn.s.barrier", "llvm.nvvm.barrier0")
		.replace("__ocml_exp_f64", "__nv_exp")
		.replace("__ocml_tanh_f64", "__nv_tanh")
		.replace("__ocml_cos_f64", "__nv_cos")
		.replace("__ocml_sin_f64", "__nv_sin")
		.replace("__ocml_log_f64", "__nv_log")
		.replace("define protected amdgpu_kernel", "define ptx_kernel")
		.replace(
			"attributes #0 = { nounwind \"amdgpu-flat-work-group-size\"=\"1,1024\" }",
			"attributes #0 = { nounwind }",
		);
	fs::write(&source, ir)?;
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
		source.to_str().ok_or_else(|| io::Error::other("NVIDIA IR path is not UTF-8"))?,
		"-Xclang",
		"-mlink-builtin-bitcode",
		"-Xclang",
		text(manifest, "nvidia-device-library")?,
		"-o",
	]);
	command.arg(&ptx_output);
	run(&mut command, "NVIDIA LLVM IR compiler")?;
	run(
		Command::new(text(manifest, "nvidia-assembler")?)
			.arg(&assembler_architecture)
			.arg(&ptx_output)
			.arg("-o")
			.arg(&output),
		"PTX assembler",
	)?;
	println!("cargo:rustc-env=RECIPE_NV_MODULE={}", output.display());
	println!("cargo:rustc-link-search=native={}", text(manifest, "nvidia-library")?);
	Ok(())
}

fn main() -> BuildResult<()> {
	let manifest = fs::read_to_string("Cargo.toml")?;
	for (key, environment) in [
		("epochs", "RECIPE_TRAIN_EPOCHS"),
		("learning-rate", "RECIPE_TRAIN_LEARNING_RATE"),
		("initial-weight", "RECIPE_TRAIN_INITIAL_WEIGHT"),
		("adamw-beta1", "RECIPE_ADAMW_BETA1"),
		("adamw-beta2", "RECIPE_ADAMW_BETA2"),
		("adamw-epsilon", "RECIPE_ADAMW_EPSILON"),
		("adamw-weight-decay", "RECIPE_ADAMW_WEIGHT_DECAY"),
		("kmeans-iterations", "RECIPE_KMEANS_ITERATIONS"),
		("surrogate-epochs", "RECIPE_SURROGATE_EPOCHS"),
		("surrogate-rate", "RECIPE_SURROGATE_RATE"),
		("gpu-threads", "RECIPE_GPU_THREADS"),
		("random-seed", "RECIPE_RANDOM_SEED"),
		("normalization-epsilon", "RECIPE_NORMALIZATION_EPSILON"),
		("leak-slope", "RECIPE_LEAK_SLOPE"),
		("prelu-slope", "RECIPE_PRELU_SLOPE"),
		("elu-alpha", "RECIPE_ELU_ALPHA"),
		("selu-alpha", "RECIPE_SELU_ALPHA"),
		("selu-scale", "RECIPE_SELU_SCALE"),
		("gelu-scale", "RECIPE_GELU_SCALE"),
		("gelu-cubic", "RECIPE_GELU_CUBIC"),
		("huber-threshold", "RECIPE_HUBER_THRESHOLD"),
		("output-tolerance", "RECIPE_OUTPUT_TOLERANCE"),
		("gradient-tolerance", "RECIPE_GRADIENT_TOLERANCE"),
		("backend-tolerance", "RECIPE_BACKEND_TOLERANCE"),
	] {
		println!("cargo:rustc-env={environment}={}", number(&manifest, key)?);
	}
	for (key, environment) in [("hsa-runtime", "RECIPE_HSA_RUNTIME"), ("nvidia-runtime", "RECIPE_NV_RUNTIME")] {
		println!("cargo:rustc-env={environment}={}", text(&manifest, key)?);
	}
	let out = PathBuf::from(env::var_os("OUT_DIR").ok_or_else(|| io::Error::other("OUT_DIR must be configured"))?);
	if env::var_os("CARGO_FEATURE_AMD").is_some() {
		compile_amd(&manifest, &out)?;
	}
	if env::var_os("CARGO_FEATURE_NVIDIA").is_some() {
		compile_nvidia(&manifest, &out)?;
	}
	println!("cargo:rerun-if-changed=Cargo.toml");
	println!("cargo:rerun-if-changed=amd-nv.ll");
	Ok(())
}
