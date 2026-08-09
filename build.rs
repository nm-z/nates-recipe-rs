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
	let ir = ir.replace("call i32 @llvm.amdgcn.workitem.id.x()", "call i32 @global_id()").replace("call void @llvm.amdgcn.s.barrier()", "call void @grid_barrier(i32 %threads)");
	ir.replacen('\n', &format!("\n{}\n", PARALLEL.replace("declare i32 @recipe.workgroup.size.x()", width)), 1).replace("recipe.local.id.x", "llvm.amdgcn.workitem.id.x").replace("recipe.group.id.x", "llvm.amdgcn.workgroup.id.x").replace("recipe.local.barrier", "llvm.amdgcn.s.barrier")
}
fn word(text: String, from: &str, to: &str) -> String {
	let (mut output, mut rest) = (String::with_capacity(text.len()), text.as_str());
	while let Some(index) = rest.find(from) {
		let end = index + from.len();
		let identifier = |value: char| value.is_ascii_alphanumeric() || value == '_' || value == '.';
		let bounded = rest[..index].chars().next_back().is_none_or(|value| !identifier(value)) && rest[end..].chars().next().is_none_or(|value| !identifier(value));
		output.push_str(&rest[..index]);
		output.push_str(if bounded { to } else { from });
		rest = &rest[end..];
	}
	output.push_str(rest);
	output
}
fn float_ir(ir: String) -> String {
	word(ir.replace("%f32.result = fpext float %f32.value to double", "%f32.result = fadd float %f32.value, 0.0"), "double", "float").replace("@contraction_tile", "@contraction_tile_f32").replace("@forward_graph", "@forward_graph_f32").replace("@tape_epoch_graph", "@tape_epoch_graph_f32").replace("to double", "to float").replace(".f64", ".f32").replace("_f64", "_f32").replace("@__nv_exp(", "@__nv_expf(").replace("@__nv_tanh(", "@__nv_tanhf(").replace("@__nv_cos(", "@__nv_cosf(").replace("@__nv_sin(", "@__nv_sinf(").replace("@__nv_log(", "@__nv_logf(").replace("0.1", "0x3FB99999A0000000").replace("0x3CB0000000000000", "0x3E80000000000000").replace("0x3FEFFFFFFFFFFFFE", "0x3FEFFFFFE0000000").replace("align 8", "align 4")
}
fn setting<'a>(manifest: &'a str, key: &str) -> BuildResult<&'a str> {
	let prefix = format!("{key} = ");
	manifest.lines().find_map(|line| line.trim().strip_prefix(&prefix)).ok_or_else(|| io::Error::other(format!("{key} must be configured")).into())
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
	setting(manifest, key)?.strip_prefix('"').and_then(|value| value.strip_suffix('"')).ok_or_else(|| io::Error::other(format!("{key} must be quoted")).into())
}
fn architectures(manifest: &str) -> BuildResult<Vec<String>> {
	let directory = text(manifest, "hsa-device-library-directory")?;
	let mut values = Vec::new();
	for entry in fs::read_dir(directory)? {
		let name = entry?.file_name().into_string().map_err(|_| io::Error::other("ISA library name is not UTF-8"))?;
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
	let float_source = out.join("recipe-amd-f32.ll");
	let ir = parallel_ir(fs::read_to_string("amd-nv-cpu.ll")?, AMD_WIDTH);
	fs::write(&source, &ir)?;
	fs::write(&float_source, float_ir(ir))?;
	let mut objects = Vec::new();
	let mut float_objects = Vec::new();
	let mut embeds = Vec::new();
	let mut float_embeds = Vec::new();
	let mut assemblies = Vec::new();
	for architecture in architectures(manifest)? {
		let output = out.join(format!("recipe-{architecture}.hsaco"));
		let mut command = Command::new(text(manifest, "hsa-compiler")?);
		command.args(["-target", "amdgcn-amd-amdhsa", &format!("-mcpu={architecture}"), "-O2", "-nogpulib"]);
		for key in ["hsa-device-library", "hsa-clock-library", "hsa-finite-library", "hsa-math-library"] {
			command.args(["-Xclang", "-mlink-builtin-bitcode", "-Xclang", text(manifest, key)?]);
		}
		let isa = Path::new(text(manifest, "hsa-device-library-directory")?).join(format!("oclc_isa_version_{}.bc", architecture.trim_start_matches("gfx")));
		command.args(["-Xclang", "-mlink-builtin-bitcode", "-Xclang"]).arg(isa).arg(&source).arg("-o").arg(&output);
		if run(&mut command, "HSA LLVM IR compiler").is_err() {
			continue;
		}
		let float_output = out.join(format!("recipe-{architecture}-f32.hsaco"));
		let mut command = Command::new(text(manifest, "hsa-compiler")?);
		command.args(["-target", "amdgcn-amd-amdhsa", &format!("-mcpu={architecture}"), "-O2", "-nogpulib"]);
		for key in ["hsa-device-library", "hsa-clock-library", "hsa-finite-library", "hsa-math-library"] {
			command.args(["-Xclang", "-mlink-builtin-bitcode", "-Xclang", text(manifest, key)?]);
		}
		let isa = Path::new(text(manifest, "hsa-device-library-directory")?).join(format!("oclc_isa_version_{}.bc", architecture.trim_start_matches("gfx")));
		command.args(["-Xclang", "-mlink-builtin-bitcode", "-Xclang"]).arg(isa).arg(&float_source).arg("-o").arg(&float_output);
		if run(&mut command, "HSA FP32 LLVM IR compiler").is_err() {
			continue;
		}
		let assembly = out.join(format!("recipe-{architecture}.amd.s"));
		if render(Command::new(text(manifest, "hsa-disassembler")?).arg("--disassemble").arg(&output), "HSA disassembler", &assembly).is_err() {
			continue;
		}
		objects.push(format!("{architecture}={}", output.display()));
		float_objects.push(format!("{architecture}={}", float_output.display()));
		embeds.push(format!("(\"{architecture}\", include_bytes!(\"{}\").as_slice()),", output.display()));
		float_embeds.push(format!("(\"{architecture}\", include_bytes!(\"{}\").as_slice()),", float_output.display()));
		assemblies.push(format!("{architecture}={}", assembly.display()));
	}
	if objects.is_empty() {
		return Err(io::Error::other("no HSA architecture compiled").into());
	}
	let table = format!("static HSA_CODE_OBJECTS: &[(&str, &[u8])] = &[{}]\x3b static HSA_F32_CODE_OBJECTS: &[(&str, &[u8])] = &[{}]\x3b", embeds.join(" "), float_embeds.join(" "));
	fs::write(out.join("hsa-embed.rs"), table)?;
	println!("cargo:rustc-env=RECIPE_HSA_CODE_OBJECTS={}", objects.join("\x3b"));
	println!("cargo:rustc-env=RECIPE_HSA_F32_CODE_OBJECTS={}", float_objects.join("\x3b"));
	println!("cargo:rustc-env=RECIPE_HSA_ASSEMBLIES={}", assemblies.join("\x3b"));
	println!("cargo:rustc-link-search=native={}", text(manifest, "hsa-library")?);
	Ok(())
}
fn compile_nvidia(manifest: &str, out: &PathBuf) -> BuildResult<()> {
	let cuda = env::var_os("CUDA_PATH").map(PathBuf::from);
	let gpu_architecture = text(manifest, "nvidia-minimum-architecture")?;
	let architecture = format!("-march={gpu_architecture}");
	let ptx = format!("+{}", text(manifest, "nvidia-ptx")?);
	let source = out.join("recipe-nvidia.ll");
	let float_source = out.join("recipe-nvidia-f32.ll");
	let ir = parallel_ir(fs::read_to_string("amd-nv-cpu.ll")?, "declare i32 @recipe.workgroup.size.x()").replace("amdgcn-amd-amdhsa", "nvptx64-nvidia-cuda").replace("__ockl_steadyctr_u64", "llvm.nvvm.read.ptx.sreg.globaltimer").replace("llvm.amdgcn.workitem.id.x", "llvm.nvvm.read.ptx.sreg.tid.x").replace("llvm.amdgcn.workgroup.id.x", "llvm.nvvm.read.ptx.sreg.ctaid.x").replace("recipe.workgroup.size.x", "llvm.nvvm.read.ptx.sreg.ntid.x").replace("llvm.amdgcn.s.barrier", "llvm.nvvm.barrier0").replace("__ocml_exp_f64", "__nv_exp").replace("__ocml_tanh_f64", "__nv_tanh").replace("__ocml_cos_f64", "__nv_cos").replace("__ocml_sin_f64", "__nv_sin").replace("__ocml_log_f64", "__nv_log").replace("define protected amdgpu_kernel", "define ptx_kernel").replace("attributes #0 = { nounwind \"amdgpu-flat-work-group-size\"=\"1,1024\" }", "attributes #0 = { nounwind }");
	fs::write(&source, &ir)?;
	fs::write(&float_source, float_ir(ir))?;
	let compiler = text(manifest, "nvidia-compiler")?;
	let compiler = if Path::new(compiler).exists() { compiler } else { "clang" };
	let device = text(manifest, "nvidia-device-library")?;
	let device = cuda.map(|path| path.join("nvvm/libdevice/libdevice.10.bc")).filter(|path| path.exists()).unwrap_or_else(|| device.into());
	for (source, output, environment) in [(&source, out.join("recipe.ptx"), "RECIPE_NV_PTX"), (&float_source, out.join("recipe-f32.ptx"), "RECIPE_NV_F32_PTX")] {
		let mut command = Command::new(compiler);
		command.args(["-target", "nvptx64-nvidia-cuda", &architecture, "-Xclang", "-target-feature", "-Xclang", &ptx, "-O2", "-S", "-x", "ir", source.to_str().ok_or_else(|| io::Error::other("NVIDIA IR path is not UTF-8"))?, "-Xclang", "-mlink-builtin-bitcode", "-Xclang", device.to_str().ok_or_else(|| io::Error::other("NVIDIA device library path is not UTF-8"))?, "-o"]);
		command.arg(&output);
		run(&mut command, "NVIDIA LLVM IR compiler")?;
		println!("cargo:rustc-env={environment}={}", output.display());
	}
	println!("cargo:rustc-link-search=native={}", text(manifest, "nvidia-library")?);
	Ok(())
}
const CPU_REPLACEMENTS: &[(&str, &str)] = &[("@contraction_tile = external addrspace(3) global [0 x double], align 8", "@contraction_tile = internal global [65536 x double] zeroinitializer, align 8"), (" addrspace(3)", ""), ("call i32 @llvm.amdgcn.workitem.id.x()", "add i32 0, 0"), ("call i32 @recipe.local.id.x()", "add i32 0, 0"), ("call i32 @recipe.group.id.x()", "add i32 0, 0"), ("call i32 @recipe.workgroup.size.x()", "add i32 1, 0"), ("call void @llvm.amdgcn.s.barrier()", ""), ("call void @recipe.local.barrier()", ""), ("call i64 @__ockl_steadyctr_u64()", "add i64 0, 0"), ("declare i32 @llvm.amdgcn.workitem.id.x()", ""), ("declare void @llvm.amdgcn.s.barrier()", ""), ("declare i64 @__ockl_steadyctr_u64()", ""), ("__ocml_exp_f64", "exp"), ("__ocml_tanh_f64", "tanh"), ("__ocml_cos_f64", "cos"), ("__ocml_sin_f64", "sin"), ("__ocml_log_f64", "log"), ("define protected amdgpu_kernel void @forward_graph(", "define void @recipe_forward_cpu("), ("define protected amdgpu_kernel void @tape_epoch_graph(", "define void @recipe_epoch_cpu("), ("attributes #0 = { nounwind \"amdgpu-flat-work-group-size\"=\"1,1024\" }", "attributes #0 = { nounwind }")];
fn compile_cpu(manifest: &str, out: &PathBuf) -> BuildResult<()> {
	let target = env::var("TARGET")?;
	let mut ir = fs::read_to_string("amd-nv-cpu.ll")?.replace("amdgcn-amd-amdhsa", &target);
	for (pattern, replacement) in CPU_REPLACEMENTS {
		ir = ir.replace(pattern, replacement);
	}
	let clang = ["nvidia-compiler", "hsa-compiler"].iter().filter_map(|key| text(manifest, key).ok()).find(|path| Path::new(path).exists()).unwrap_or("clang");
	let float = float_ir(ir.clone()).replace("@recipe_forward_cpu(", "@recipe_forward_cpu_f32(").replace("@recipe_epoch_cpu(", "@recipe_epoch_cpu_f32(").replace("@exp(", "@expf(").replace("@tanh(", "@tanhf(").replace("@cos(", "@cosf(").replace("@sin(", "@sinf(").replace("@log(", "@logf(");
	let mut objects = Vec::new();
	for (name, contents) in [("recipe-cpu", ir), ("recipe-cpu-f32", float)] {
		let source = out.join(format!("{name}.ll"));
		let object = out.join(format!("{name}.o"));
		fs::write(&source, contents)?;
		let mut compile = Command::new(clang);
		compile.args(["-target", &target, "-Xclang", "-opaque-pointers", "-x", "ir", "-O2", "-fPIC", "-c", "-o"]).arg(&object).arg(&source);
		if run(&mut compile, "CPU LLVM IR compiler").is_err() {
			let mut backend = Command::new("llc");
			backend.args(["--mtriple", &target, "-O2", "--relocation-model=pic", "-filetype=obj", "-o"]).arg(&object).arg(&source);
			run(&mut backend, "CPU LLVM IR backend")?;
		}
		objects.push(object);
	}
	let mut archive = Command::new("llvm-ar");
	archive.arg("rcs").arg(out.join("librecipe_cpu.a")).args(objects);
	run(&mut archive, "CPU archive")?;
	println!("cargo:rustc-link-search=native={}", out.display());
	println!("cargo:rustc-link-lib=static=recipe_cpu");
	if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux") && env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("musl") {
		println!("cargo:rustc-link-lib=m");
	}
	Ok(())
}
fn main() -> BuildResult<()> {
	let manifest = fs::read_to_string("Cargo.toml")?;
	for (key, environment) in [("epochs", "RECIPE_TRAIN_EPOCHS"), ("learning-rate", "RECIPE_TRAIN_LEARNING_RATE"), ("initial-weight", "RECIPE_TRAIN_INITIAL_WEIGHT"), ("adamw-beta1", "RECIPE_ADAMW_BETA1"), ("adamw-beta2", "RECIPE_ADAMW_BETA2"), ("adamw-epsilon", "RECIPE_ADAMW_EPSILON"), ("adamw-weight-decay", "RECIPE_ADAMW_WEIGHT_DECAY"), ("kmeans-iterations", "RECIPE_KMEANS_ITERATIONS"), ("quantization-block-weights", "RECIPE_QUANTIZATION_BLOCK_WEIGHTS"), ("surrogate-epochs", "RECIPE_SURROGATE_EPOCHS"), ("surrogate-rate", "RECIPE_SURROGATE_RATE"), ("surrogate-width", "RECIPE_SURROGATE_WIDTH"), ("rat-batch-rows", "RECIPE_RAT_BATCH_ROWS"), ("topology-latency-bytes", "RECIPE_TOPOLOGY_LATENCY_BYTES"), ("topology-bandwidth-bytes", "RECIPE_TOPOLOGY_BANDWIDTH_BYTES"), ("topology-probe-repetitions", "RECIPE_TOPOLOGY_REPETITIONS"), ("ssh-connect-timeout-seconds", "RECIPE_SSH_CONNECT_TIMEOUT"), ("random-seed", "RECIPE_RANDOM_SEED"), ("training-trials", "RECIPE_TRAINING_TRIALS"), ("training-start-run", "RECIPE_TRAINING_START_RUN"), ("normalization-epsilon", "RECIPE_NORMALIZATION_EPSILON"), ("leak-slope", "RECIPE_LEAK_SLOPE"), ("prelu-slope", "RECIPE_PRELU_SLOPE"), ("elu-alpha", "RECIPE_ELU_ALPHA"), ("selu-alpha", "RECIPE_SELU_ALPHA"), ("selu-scale", "RECIPE_SELU_SCALE"), ("gelu-scale", "RECIPE_GELU_SCALE"), ("gelu-cubic", "RECIPE_GELU_CUBIC"), ("huber-threshold", "RECIPE_HUBER_THRESHOLD"), ("output-tolerance", "RECIPE_OUTPUT_TOLERANCE"), ("gradient-tolerance", "RECIPE_GRADIENT_TOLERANCE"), ("backend-tolerance", "RECIPE_BACKEND_TOLERANCE")] {
		println!("cargo:rustc-env={environment}={}", number(&manifest, key)?);
	}
	for (key, environment) in [("hsa-runtime", "RECIPE_HSA_RUNTIME"), ("nvidia-runtime", "RECIPE_NV_RUNTIME"), ("ssh-config", "RECIPE_SSH_CONFIG")] {
		println!("cargo:rustc-env={environment}={}", text(&manifest, key)?);
	}
	println!("cargo:rustc-env=RECIPE_MULTI_DEVICE={}", flag(&manifest, "multi-device")?);
	let out = PathBuf::from(env::var_os("OUT_DIR").ok_or_else(|| io::Error::other("OUT_DIR must be configured"))?);
	println!("cargo::rustc-check-cfg=cfg(amd)");
	println!("cargo::rustc-check-cfg=cfg(nvidia)");
	let toolchain = |compiler: &str, library: &str| -> BuildResult<bool> { Ok(Path::new(text(&manifest, compiler)?).exists() && Path::new(text(&manifest, library)?).exists()) };
	compile_cpu(&manifest, &out)?;
	// GPU driver stubs and library search paths are host-arch: cross-compiled builds are CPU-only.
	let native = env::var("TARGET")? == env::var("HOST")?;
	let amd = native && toolchain("hsa-compiler", "hsa-device-library")?;
	let nvidia = native && (toolchain("nvidia-compiler", "nvidia-device-library")? || env::var_os("CUDA_PATH").is_some());
	if amd {
		println!("cargo:rustc-cfg=amd");
		compile_amd(&manifest, &out)?;
	}
	if nvidia {
		println!("cargo:rustc-cfg=nvidia");
		compile_nvidia(&manifest, &out)?;
	}
	println!("cargo:rerun-if-changed=Cargo.toml");
	println!("cargo:rerun-if-changed=amd-nv-cpu.ll");
	Ok(())
}
