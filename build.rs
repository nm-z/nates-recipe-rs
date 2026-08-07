use std::{env, error::Error, fs, io, path::{Path, PathBuf}, process::Command};
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
} fn setting<'a>(manifest: &'a str, key: &str) -> BuildResult<&'a str> {
	let prefix = format!("{key} = ");
	manifest
		.lines()
		.find_map(|line| line.trim().strip_prefix(&prefix))
			.ok_or_else(|| io::Error::other(format!("{key} must be configured")).into())
} fn number<'a>(manifest: &'a str, key: &str) -> BuildResult<&'a str> { 	let value = setting(manifest, key)?;
	value.parse::<f64>().map_err(|error| io::Error::other(format!("{key} must be numeric: {error}")))?;
	Ok(value)
} fn natural<'a>(manifest: &'a str, key: &str) -> BuildResult<&'a str> { 	let value = setting(manifest, key)?;
	value.parse::<u32>().map_err(|error| io::Error::other(format!("{key} must be a natural number: {error}")))?;
	Ok(value)
} fn text<'a>(manifest: &'a str, key: &str) -> BuildResult<&'a str> {
	setting(manifest, key)?
		.strip_prefix('"')
		.and_then(|value| value.strip_suffix('"'))
		.ok_or_else(|| io::Error::other(format!("{key} must be quoted")).into())
} fn run(command: &mut Command, role: &str) -> BuildResult<()> {
	let status = command.status()?;
	if !status.success() {
		return Err(io::Error::other(format!("{role} failed")).into());
	}
	Ok(())
} fn render(command: &mut Command, role: &str, path: &Path) -> BuildResult<()> {
	let output = command.output()?;
	if !output.status.success() {
		let detail = String::from_utf8_lossy(&output.stderr);
		return Err(io::Error::other(format!("{role} failed: {}", detail.trim())).into());
	}
	fs::write(path, output.stdout)?;
	Ok(())
} fn amd_resources(manifest: &str, object: &Path) -> BuildResult<()> {
	let tool = Path::new(text(manifest, "hsa-disassembler")?).with_file_name("llvm-readobj");
	let output = Command::new(tool).arg("--notes").arg(object).output()?;
	if !output.status.success() { return Err(io::Error::other("HSA metadata reader failed").into()); }
	let notes = String::from_utf8(output.stdout)?;
	for (kernel, prefix) in [("forward_graph", "FORWARD"), ("tape_epoch_graph", "EPOCH")] {
		let name = format!(".name:           {kernel}");
		let section = notes.split("  - .args:").find(|value| value.contains(&name))
			.ok_or_else(|| io::Error::other(format!("HSA metadata lacks {kernel}")))?;
		for (field, suffix) in [(".vgpr_count:", "VGPRS"), (".max_flat_workgroup_size:", "MAX_BLOCK")] {
			let value = section.lines().find_map(|line| line.trim().strip_prefix(field))
				.ok_or_else(|| io::Error::other(format!("HSA metadata lacks {field}")))?;
			println!("cargo:rustc-env=RECIPE_HSA_{prefix}_{suffix}={}", value.trim());
		} 	} 	Ok(())
} fn compile_amd(manifest: &str, out: &PathBuf) -> BuildResult<()> {
		let architecture = text(manifest, "hsa-architecture")?;
		println!("cargo:rustc-env=RECIPE_HSA_ARCHITECTURE={architecture}");
	let cpu = format!("-mcpu={architecture}");
	let output = out.join("recipe.hsaco");
	let source = out.join("recipe-amd.ll");
	let ir = parallel_ir(fs::read_to_string("amd-nv.ll")?, AMD_WIDTH)
		.replace("RECIPE_CONTRACTION_TILE_K_MAX", natural(manifest, "contraction-tile-k-max")?);
	fs::write(&source, ir)?;
	let mut command = Command::new(text(manifest, "hsa-compiler")?);
	command.args(["-target", "amdgcn-amd-amdhsa", &cpu, "-O2", "-nogpulib"]);
	for key in ["hsa-device-library", "hsa-isa-library", "hsa-finite-library", "hsa-math-library"] {
		command.args(["-Xclang", "-mlink-builtin-bitcode", "-Xclang", text(manifest, key)?]);
	}
	command.arg(&source).arg("-o").arg(&output);
	run(&mut command, "HSA LLVM IR compiler")?;
	amd_resources(manifest, &output)?;
	let assembly = out.join("recipe.amd.s");
	render(
		Command::new(text(manifest, "hsa-disassembler")?).arg("--disassemble").arg(&output),
		"HSA disassembler",
		&assembly,
	)?;
	println!("cargo:rustc-env=RECIPE_HSA_CODE_OBJECT={}", output.display());
	println!("cargo:rustc-env=RECIPE_HSA_ASSEMBLY={}", assembly.display());
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
	let ir = parallel_ir(fs::read_to_string("amd-nv.ll")?, "declare i32 @recipe.workgroup.size.x()")
		.replace("RECIPE_CONTRACTION_TILE_K_MAX", natural(manifest, "contraction-tile-k-max")?)
		.replace("amdgcn-amd-amdhsa", "nvptx64-nvidia-cuda")
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
	run(
		Command::new(text(manifest, "nvidia-assembler")?)
			.arg(&assembler_architecture)
			.arg(&ptx_output)
			.arg("-o")
			.arg(&output),
		"PTX assembler",
	)?;
	let sass_output = out.join("recipe.nvidia.sass");
	render(Command::new(text(manifest, "nvidia-disassembler")?).arg(&output), "NVIDIA disassembler", &sass_output)?;
	println!("cargo:rustc-env=RECIPE_NV_PTX={}", ptx_output.display());
	println!("cargo:rustc-env=RECIPE_NV_MODULE={}", output.display());
	println!("cargo:rustc-env=RECIPE_NV_SASS={}", sass_output.display());
	println!("cargo:rustc-link-search=native={}", text(manifest, "nvidia-library")?);
	Ok(())
}
fn main() -> BuildResult<()> {
	let manifest = fs::read_to_string("Cargo.toml")?;
	for (key, environment) in [
		("epochs", "RECIPE_TRAIN_EPOCHS"), ("learning-rate", "RECIPE_TRAIN_LEARNING_RATE"),
		("initial-weight", "RECIPE_TRAIN_INITIAL_WEIGHT"), ("adamw-beta1", "RECIPE_ADAMW_BETA1"),
		("adamw-beta2", "RECIPE_ADAMW_BETA2"), ("adamw-epsilon", "RECIPE_ADAMW_EPSILON"),
		("adamw-weight-decay", "RECIPE_ADAMW_WEIGHT_DECAY"), ("kmeans-iterations", "RECIPE_KMEANS_ITERATIONS"),
		("surrogate-epochs", "RECIPE_SURROGATE_EPOCHS"), ("surrogate-rate", "RECIPE_SURROGATE_RATE"),
		("rat-batch-rows", "RECIPE_RAT_BATCH_ROWS"), ("contraction-tile-m", "RECIPE_TILE_M"),
		("contraction-tile-n", "RECIPE_TILE_N"), ("contraction-tile-k", "RECIPE_TILE_K"),
		("contraction-tile-m-max", "RECIPE_TILE_M_MAX"), ("contraction-tile-n-max", "RECIPE_TILE_N_MAX"),
		("contraction-tile-k-max", "RECIPE_TILE_K_MAX"),
		("random-seed", "RECIPE_RANDOM_SEED"), ("normalization-epsilon", "RECIPE_NORMALIZATION_EPSILON"),
		("leak-slope", "RECIPE_LEAK_SLOPE"), ("prelu-slope", "RECIPE_PRELU_SLOPE"),
		("elu-alpha", "RECIPE_ELU_ALPHA"), ("selu-alpha", "RECIPE_SELU_ALPHA"),
		("selu-scale", "RECIPE_SELU_SCALE"), ("gelu-scale", "RECIPE_GELU_SCALE"),
		("gelu-cubic", "RECIPE_GELU_CUBIC"), ("huber-threshold", "RECIPE_HUBER_THRESHOLD"),
		("output-tolerance", "RECIPE_OUTPUT_TOLERANCE"), ("gradient-tolerance", "RECIPE_GRADIENT_TOLERANCE"),
		("backend-tolerance", "RECIPE_BACKEND_TOLERANCE"),
	] {
		println!("cargo:rustc-env={environment}={}", number(&manifest, key)?);
	}
	for (key, environment) in [("hsa-runtime", "RECIPE_HSA_RUNTIME"), ("nvidia-runtime", "RECIPE_NV_RUNTIME")] {
		println!("cargo:rustc-env={environment}={}", text(&manifest, key)?);
	}
	let out = PathBuf::from(env::var_os("OUT_DIR").ok_or_else(|| io::Error::other("OUT_DIR must be configured"))?);
	if env::var_os("CARGO_FEATURE_AMD").is_some() { compile_amd(&manifest, &out)?; }
	if env::var_os("CARGO_FEATURE_NVIDIA").is_some() { compile_nvidia(&manifest, &out)?; }
	println!("cargo:rerun-if-changed=Cargo.toml");
	println!("cargo:rerun-if-changed=amd-nv.ll");
	Ok(())
}
