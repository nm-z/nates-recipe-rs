use std::{
	env,
	error::Error,
	fs, io,
	path::{Path, PathBuf},
	process::Command,
};
type BuildResult<T> = Result<T, Box<dyn Error>>;
const PARALLEL: &str = r#"@grid.count = internal addrspace(1) global i32 0, align 4
@grid.phase = internal addrspace(1) global i32 0, align 4
declare i32 @llvm.amdgcn.workgroup.id.x()
declare i32 @recipe.workgroup.size.x()
define internal i32 @global_id() #1 { entry:
%lane = call i32 @llvm.amdgcn.workitem.id.x() %group = call i32 @llvm.amdgcn.workgroup.id.x()
%width = call i32 @recipe.workgroup.size.x() %base = mul i32 %group, %width %id = add i32 %base, %lane ret i32 %id }
define internal void @grid_barrier(i32 %threads) #1 { entry: call void @llvm.amdgcn.s.barrier()
%lane = call i32 @llvm.amdgcn.workitem.id.x() %leader = icmp eq i32 %lane, 0
br i1 %leader, label %arrive, label %joined arrive: %width = call i32 @recipe.workgroup.size.x()
%groups = udiv i32 %threads, %width %phase = load atomic i32, ptr addrspace(1) @grid.phase acquire, align 4
%prior = atomicrmw add ptr addrspace(1) @grid.count, i32 1 acq_rel %limit = sub i32 %groups, 1
%last = icmp eq i32 %prior, %limit br i1 %last, label %release, label %wait release:
store atomic i32 0, ptr addrspace(1) @grid.count release, align 4 %next = xor i32 %phase, 1
store atomic i32 %next, ptr addrspace(1) @grid.phase release, align 4 br label %joined wait:
%seen = load atomic i32, ptr addrspace(1) @grid.phase acquire, align 4 %ready = icmp ne i32 %seen, %phase
br i1 %ready, label %joined, label %wait joined: call void @llvm.amdgcn.s.barrier() ret void }"#;
const AMD_WIDTH: &str = r#"declare ptr addrspace(4) @llvm.amdgcn.dispatch.ptr()
define internal i32 @recipe.workgroup.size.x() #1 { entry: %args = call ptr addrspace(4) @llvm.amdgcn.dispatch.ptr()
%address = getelementptr i8, ptr addrspace(4) %args, i32 4 %value = load i16, ptr addrspace(4) %address, align 2
%width = zext i16 %value to i32 ret i32 %width }"#;
fn parallel_ir(ir: String, width: &str) -> String {
	let ir = ir
		.replace("call i32 @llvm.amdgcn.workitem.id.x()", "call i32 @global_id()")
		.replace("call void @llvm.amdgcn.s.barrier()", "call void @grid_barrier(i32 %threads)");
	ir.replacen('\n', &format!("\n{}\n", PARALLEL.replace("declare i32 @recipe.workgroup.size.x()", width)), 1)
		.replace("recipe.local.id.x", "llvm.amdgcn.workitem.id.x")
		.replace("recipe.group.id.x", "llvm.amdgcn.workgroup.id.x")
		.replace("recipe.local.barrier", "llvm.amdgcn.s.barrier")
}
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
fn flag<'a>(manifest: &'a str, key: &str) -> BuildResult<&'a str> {
	let value = setting(manifest, key)?;
	value.parse::<bool>().map_err(|error| io::Error::other(format!("{key} must be true or false: {error}")))?;
	Ok(value)
}
fn text<'a>(manifest: &'a str, key: &str) -> BuildResult<&'a str> {
	setting(manifest, key)?
		.strip_prefix('"')
		.and_then(|value| value.strip_suffix('"'))
		.ok_or_else(|| io::Error::other(format!("{key} must be quoted")).into())
}
fn architectures(manifest: &str) -> BuildResult<Vec<String>> {
	let directory = text(manifest, "hsa-device-library-directory")?;
	let mut values = Vec::new();
	for entry in fs::read_dir(directory)? {
		let name =
			entry?.file_name().into_string().map_err(|_| io::Error::other("ISA library name is not UTF-8"))?;
		if let Some(version) = name.strip_prefix("oclc_isa_version_").and_then(|value| value.strip_suffix(".bc"))
			&& !version.contains('-')
		{
			values.push(format!("gfx{version}"));
		}
	}
	values.sort();
	values.dedup();
	if values.is_empty() {
		return Err(io::Error::other("hsa-device-library-directory has no ISA libraries").into());
	}
	Ok(values)
}
fn run(command: &mut Command, role: &str) -> BuildResult<()> {
	let status = command.status()?;
	if !status.success() {
		return Err(io::Error::other(format!("{role} failed")).into());
	}
	Ok(())
}
fn render(command: &mut Command, role: &str, path: &Path) -> BuildResult<()> {
	let output = command.output()?;
	if !output.status.success() {
		let detail = String::from_utf8_lossy(&output.stderr);
		return Err(io::Error::other(format!("{role} failed: {}", detail.trim())).into());
	}
	fs::write(path, output.stdout)?;
	Ok(())
}
fn compile_amd(manifest: &str, out: &PathBuf) -> BuildResult<()> {
	let source = out.join("recipe-amd.ll");
	let ir = parallel_ir(fs::read_to_string("amd-nv.ll")?, AMD_WIDTH);
	fs::write(&source, ir)?;
	let mut objects = Vec::new();
	let mut embeds = Vec::new();
	let mut assemblies = Vec::new();
	for architecture in architectures(manifest)? {
		let output = out.join(format!("recipe-{architecture}.hsaco"));
		let mut command = Command::new(text(manifest, "hsa-compiler")?);
		command.args(["-target", "amdgcn-amd-amdhsa", &format!("-mcpu={architecture}"), "-O2", "-nogpulib"]);
		for key in ["hsa-device-library", "hsa-clock-library", "hsa-finite-library", "hsa-math-library"] {
			command.args(["-Xclang", "-mlink-builtin-bitcode", "-Xclang", text(manifest, key)?]);
		}
		let isa = Path::new(text(manifest, "hsa-device-library-directory")?)
			.join(format!("oclc_isa_version_{}.bc", architecture.trim_start_matches("gfx")));
		command.args(["-Xclang", "-mlink-builtin-bitcode", "-Xclang"]).arg(isa).arg(&source).arg("-o").arg(&output);
		if run(&mut command, "HSA LLVM IR compiler").is_err() {
			println!("cargo:warning=skipped {architecture}: the HSA kernel does not compile for it");
			continue;
		}
		let assembly = out.join(format!("recipe-{architecture}.amd.s"));
		if render(
			Command::new(text(manifest, "hsa-disassembler")?).arg("--disassemble").arg(&output),
			"HSA disassembler",
			&assembly,
		)
		.is_err()
		{
			println!("cargo:warning=skipped {architecture}: the HSA object does not disassemble");
			continue;
		}
		objects.push(format!("{architecture}={}", output.display()));
		embeds.push(format!("(\"{architecture}\", include_bytes!(\"{}\").as_slice()),", output.display()));
		assemblies.push(format!("{architecture}={}", assembly.display()));
	}
	if objects.is_empty() {
		return Err(io::Error::other("no HSA architecture compiled").into());
	}
	let table = format!("static HSA_CODE_OBJECTS: &[(&str, &[u8])] = &[{}]\x3b", embeds.join(" "));
	fs::write(out.join("hsa-embed.rs"), table)?;
	println!("cargo:rustc-env=RECIPE_HSA_CODE_OBJECTS={}", objects.join("\x3b"));
	println!("cargo:rustc-env=RECIPE_HSA_ASSEMBLIES={}", assemblies.join("\x3b"));
	println!("cargo:rustc-link-search=native={}", text(manifest, "hsa-library")?);
	Ok(())
}
fn compile_nvidia(manifest: &str, out: &PathBuf) -> BuildResult<()> {
	let gpu_architecture = text(manifest, "nvidia-minimum-architecture")?;
	let architecture = format!("-march={gpu_architecture}");
	let ptx = format!("+{}", text(manifest, "nvidia-ptx")?);
	let ptx_output = out.join("recipe.ptx");
	let source = out.join("recipe-nvidia.ll");
	let ir = parallel_ir(fs::read_to_string("amd-nv.ll")?, "declare i32 @recipe.workgroup.size.x()")
		.replace("amdgcn-amd-amdhsa", "nvptx64-nvidia-cuda")
		.replace("__ockl_steadyctr_u64", "llvm.nvvm.read.ptx.sreg.globaltimer")
		.replace("llvm.amdgcn.workitem.id.x", "llvm.nvvm.read.ptx.sreg.tid.x")
		.replace("llvm.amdgcn.workgroup.id.x", "llvm.nvvm.read.ptx.sreg.ctaid.x")
		.replace("recipe.workgroup.size.x", "llvm.nvvm.read.ptx.sreg.ntid.x")
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
	println!("cargo:rustc-env=RECIPE_NV_PTX={}", ptx_output.display());
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
		("surrogate-width", "RECIPE_SURROGATE_WIDTH"),
		("rat-batch-rows", "RECIPE_RAT_BATCH_ROWS"),
		("topology-latency-bytes", "RECIPE_TOPOLOGY_LATENCY_BYTES"),
		("topology-bandwidth-bytes", "RECIPE_TOPOLOGY_BANDWIDTH_BYTES"),
		("topology-probe-repetitions", "RECIPE_TOPOLOGY_REPETITIONS"),
		("ssh-connect-timeout-seconds", "RECIPE_SSH_CONNECT_TIMEOUT"),
		("random-seed", "RECIPE_RANDOM_SEED"),
		("training-trials", "RECIPE_TRAINING_TRIALS"),
		("training-start-run", "RECIPE_TRAINING_START_RUN"),
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
	for (key, environment) in [
		("hsa-runtime", "RECIPE_HSA_RUNTIME"),
		("nvidia-runtime", "RECIPE_NV_RUNTIME"),
		("ssh-config", "RECIPE_SSH_CONFIG"),
	] {
		println!("cargo:rustc-env={environment}={}", text(&manifest, key)?);
	}
	println!("cargo:rustc-env=RECIPE_MULTI_DEVICE={}", flag(&manifest, "multi-device")?);
	let out = PathBuf::from(env::var_os("OUT_DIR").ok_or_else(|| io::Error::other("OUT_DIR must be configured"))?);
	println!("cargo::rustc-check-cfg=cfg(amd)");
	println!("cargo::rustc-check-cfg=cfg(nvidia)");
	let toolchain = |compiler: &str, library: &str| -> BuildResult<bool> {
		Ok(Path::new(text(&manifest, compiler)?).exists() && Path::new(text(&manifest, library)?).exists())
	};
	let amd = toolchain("hsa-compiler", "hsa-device-library")?;
	let nvidia = toolchain("nvidia-compiler", "nvidia-device-library")?;
	if !amd && !nvidia {
		return Err(io::Error::other("no ROCm or CUDA toolchain is installed on the build machine").into());
	}
	if amd {
		println!("cargo:rustc-cfg=amd");
		compile_amd(&manifest, &out)?;
	}
	if nvidia {
		println!("cargo:rustc-cfg=nvidia");
		compile_nvidia(&manifest, &out)?;
	}
	println!("cargo:rerun-if-changed=Cargo.toml");
	println!("cargo:rerun-if-changed=amd-nv.ll");
	Ok(())
}
