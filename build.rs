use std::{env, fs, path::PathBuf, process::Command};

const HSA_IR: &str = include_str!("amd.ll");
const NV_IR: &str = include_str!("nv.ll");

fn main() {
	let manifest = fs::read_to_string("Cargo.toml")
		.unwrap_or_else(|error| panic!("Cargo.toml must be readable: {error}"));
	for (key, environment) in [
		("balance-bytes-per-flop", "RECIPE_BALANCE_BYTES_PER_FLOP"),
		("epochs", "RECIPE_TRAIN_EPOCHS"),
		("learning-rate", "RECIPE_TRAIN_LEARNING_RATE"),
		(
			"finite-difference-step",
			"RECIPE_TRAIN_FINITE_DIFFERENCE_STEP",
		),
		("initial-weight", "RECIPE_TRAIN_INITIAL_WEIGHT"),
		("input-features", "RECIPE_TRAIN_INPUT_FEATURES"),
		("hidden-neurons", "RECIPE_TRAIN_HIDDEN_NEURONS"),
		("output-neurons", "RECIPE_TRAIN_OUTPUT_NEURONS"),
		("maximum-final-loss", "RECIPE_TRAIN_MAXIMUM_FINAL_LOSS"),
		("adamw-beta1", "RECIPE_TRAIN_ADAMW_BETA1"),
		("adamw-beta2", "RECIPE_TRAIN_ADAMW_BETA2"),
		("adamw-epsilon", "RECIPE_TRAIN_ADAMW_EPSILON"),
		("adamw-weight-decay", "RECIPE_TRAIN_ADAMW_WEIGHT_DECAY"),
		("leak-slope", "RECIPE_LEAK_SLOPE"),
		("prelu-initial-slope", "RECIPE_PRELU_INITIAL_SLOPE"),
		("elu-alpha", "RECIPE_ELU_ALPHA"),
		("selu-alpha", "RECIPE_SELU_ALPHA"),
		("selu-scale", "RECIPE_SELU_SCALE"),
		("gelu-scale", "RECIPE_GELU_SCALE"),
		("gelu-cubic", "RECIPE_GELU_CUBIC"),
		("normalization-epsilon", "RECIPE_NORMALIZATION_EPSILON"),
		("gpu-threads", "RECIPE_GPU_THREADS"),
		("random-seed", "RECIPE_RANDOM_SEED"),
	] {
		let prefix = format!("{key} = ");
		let value = manifest
			.lines()
			.find_map(|line| line.trim().strip_prefix(&prefix))
			.unwrap_or_else(|| panic!("{key} must be configured"));
		value.parse::<f64>()
			.unwrap_or_else(|_| panic!("{key} must be numeric"));
		println!("cargo:rustc-env={environment}={value}");
	}
	let architecture = manifest
		.lines()
		.find_map(|line| {
			line.trim()
				.strip_prefix("hsa-architecture = \"")
				.and_then(|value| value.strip_suffix('"'))
		})
		.unwrap_or_else(|| panic!("hsa-architecture must be configured"));
	let compiler = manifest
		.lines()
		.find_map(|line| {
			line.trim()
				.strip_prefix("hsa-compiler = \"")
				.and_then(|value| value.strip_suffix('"'))
		})
		.unwrap_or_else(|| panic!("hsa-compiler must be configured"));
	let library = manifest
		.lines()
		.find_map(|line| {
			line.trim()
				.strip_prefix("hsa-library = \"")
				.and_then(|value| value.strip_suffix('"'))
		})
		.unwrap_or_else(|| panic!("hsa-library must be configured"));
	let device_library = manifest
		.lines()
		.find_map(|line| {
			line.trim()
				.strip_prefix("hsa-device-library = \"")
				.and_then(|value| value.strip_suffix('"'))
		})
		.unwrap_or_else(|| panic!("hsa-device-library must be configured"));
	let isa_library = manifest
		.lines()
		.find_map(|line| {
			line.trim()
				.strip_prefix("hsa-isa-library = \"")
				.and_then(|value| value.strip_suffix('"'))
		})
		.unwrap_or_else(|| panic!("hsa-isa-library must be configured"));
	let finite_library = manifest
		.lines()
		.find_map(|line| {
			line.trim()
				.strip_prefix("hsa-finite-library = \"")
				.and_then(|value| value.strip_suffix('"'))
		})
		.unwrap_or_else(|| panic!("hsa-finite-library must be configured"));
	let math_library = manifest
		.lines()
		.find_map(|line| {
			line.trim()
				.strip_prefix("hsa-math-library = \"")
				.and_then(|value| value.strip_suffix('"'))
		})
		.unwrap_or_else(|| panic!("hsa-math-library must be configured"));
	let configured = |key: &str| {
		manifest
			.lines()
			.find_map(|line| {
				line.trim()
					.strip_prefix(&format!("{key} = \""))
					.and_then(|value| value.strip_suffix('"'))
			})
			.unwrap_or_else(|| panic!("{key} must be configured"))
	};
	let (nv_architecture, nv_ptx, nv_compiler, nv_library, nv_device_library) = (
		configured("nvidia-architecture"),
		configured("nvidia-ptx"),
		configured("nvidia-compiler"),
		configured("nvidia-library"),
		configured("nvidia-device-library"),
	);
	let out = PathBuf::from(
		env::var_os("OUT_DIR").unwrap_or_else(|| panic!("OUT_DIR must be configured")),
	);
	let source = out.join("recipe_hsa.ll");
	fs::write(&source, HSA_IR)
		.unwrap_or_else(|error| panic!("cannot write HSA LLVM IR: {error}"));
	let code_object = out.join("recipe.hsaco");
	let cpu = format!("-mcpu={architecture}");
	let mut command = Command::new(compiler);
	command.args(["-target", "amdgcn-amd-amdhsa", &cpu, "-O2", "-nogpulib"]);
	for bitcode in [device_library, isa_library, finite_library, math_library] {
		command.args(["-Xclang", "-mlink-builtin-bitcode", "-Xclang", bitcode]);
	}
	let status = command
		.arg(&source)
		.arg("-o")
		.arg(&code_object)
		.status()
		.unwrap_or_else(|error| panic!("cannot run clang: {error}"));
	if !status.success() {
		panic!("HSA LLVM IR compilation failed");
	}
	let nv_source = out.join("recipe_nv.ll");
	fs::write(&nv_source, NV_IR)
		.unwrap_or_else(|error| panic!("cannot write NVIDIA LLVM IR: {error}"));
	let nv_module = out.join("recipe.ptx");
	let nv_cpu = format!("-march={nv_architecture}");
	let nv_feature = format!("+{nv_ptx}");
	let status = Command::new(nv_compiler)
		.args([
			"-target",
			"nvptx64-nvidia-cuda",
			&nv_cpu,
			"-Xclang",
			"-target-feature",
			"-Xclang",
			&nv_feature,
			"-O2",
			"-S",
			"-x",
			"ir",
		])
		.arg(&nv_source)
		.args([
			"-Xclang",
			"-mlink-builtin-bitcode",
			"-Xclang",
			nv_device_library,
			"-o",
		])
		.arg(&nv_module)
		.status()
		.unwrap_or_else(|error| panic!("cannot run NVIDIA clang: {error}"));
	if !status.success() {
		panic!("NVIDIA LLVM IR compilation failed");
	}
	println!(
		"cargo:rustc-env=RECIPE_HSA_CODE_OBJECT={}",
		code_object.display()
	);
	println!("cargo:rustc-env=RECIPE_NV_MODULE={}", nv_module.display());
	println!("cargo:rustc-link-search=native={library}");
	println!("cargo:rustc-link-search=native={nv_library}");
	println!("cargo:rerun-if-changed=Cargo.toml");
	println!("cargo:rerun-if-changed=amd.ll");
	println!("cargo:rerun-if-changed=nv.ll");
}
