use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::{
	AmdTarget, ArtifactDigest, InspectedCubin, InspectedHsaco, KernelTarget, LoweredKernel, LoweringError,
	LoweringErrorKind, NvidiaTarget, inspect_cubin, inspect_hsaco,
};

static WORKSPACE_NONCE: AtomicU64 = AtomicU64::new(0);
const ERROR_OUTPUT_LIMIT: usize = 16 * 1024;

/// The only phases in which Recipe may invoke an artifact compiler.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuildPhase {
	Offline,
	Realize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PinnedTool {
	pub path: PathBuf,
	pub digest: ArtifactDigest,
}

impl PinnedTool {
	/// Capture the current executable identity for a subsequently hashed build
	/// recipe. Deployment policy may instead construct this with a separately
	/// approved digest.
	pub fn inspect(path: impl AsRef<Path>) -> Result<Self, LoweringError> {
		let path = canonical_tool(path.as_ref())?;
		let bytes = fs::read(&path).map_err(|error| io_error("read tool", &path, error))?;
		Ok(Self {
			path,
			digest: ArtifactDigest::of(&bytes),
		})
	}

	fn verify(&self) -> Result<(), LoweringError> {
		let path = canonical_tool(&self.path)?;
		if path != self.path {
			return Err(LoweringError::new(
				LoweringErrorKind::ArtifactMismatch,
				format!(
					"tool path `{}` resolves to `{}`",
					self.path.display(),
					path.display()
				),
			));
		}
		let bytes = fs::read(&path).map_err(|error| io_error("read tool", &path, error))?;
		let actual = ArtifactDigest::of(&bytes);
		if actual != self.digest {
			return Err(LoweringError::new(
				LoweringErrorKind::ArtifactMismatch,
				format!(
					"tool `{}` digest {} does not match pinned {}",
					path.display(),
					actual.to_hex(),
					self.digest.to_hex()
				),
			));
		}
		Ok(())
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OfflineToolchain {
	pub verifier: PinnedTool,
	pub llvm_codegen: PinnedTool,
	pub elf_linker: Option<PinnedTool>,
	pub ptx_assembler: Option<PinnedTool>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolInvocation {
	pub program: PathBuf,
	pub arguments: Vec<OsString>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildProvenance {
	pub phase: BuildPhase,
	pub llvm_ir: ArtifactDigest,
	pub tools: Vec<(PathBuf, ArtifactDigest)>,
	pub invocations: Vec<ToolInvocation>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BuiltArtifact {
	Hsaco {
		bytes: Vec<u8>,
		inspection: InspectedHsaco,
		provenance: BuildProvenance,
	},
	Cubin {
		ptx: Vec<u8>,
		bytes: Vec<u8>,
		inspection: InspectedCubin,
		provenance: BuildProvenance,
	},
}

#[derive(Clone, Debug)]
pub struct ArtifactBuilder {
	toolchain: OfflineToolchain,
}

impl ArtifactBuilder {
	pub fn new(toolchain: OfflineToolchain) -> Result<Self, LoweringError> {
		toolchain.verifier.verify()?;
		toolchain.llvm_codegen.verify()?;
		if let Some(linker) = &toolchain.elf_linker {
			linker.verify()?;
		}
		if let Some(assembler) = &toolchain.ptx_assembler {
			assembler.verify()?;
		}
		Ok(Self { toolchain })
	}

	/// Compile and structurally inspect one already-lowered kernel.
	///
	/// `scratch_parent` is explicit and must be an existing private directory.
	/// No compiler entry point exists for Finalize, init, loop, or exit.
	pub fn build(
		&self,
		phase: BuildPhase,
		lowered: &LoweredKernel,
		target: &KernelTarget,
		scratch_parent: &Path,
	) -> Result<BuiltArtifact, LoweringError> {
		target.validate()?;
		if &lowered.target != target {
			return Err(LoweringError::new(
				LoweringErrorKind::ArtifactMismatch,
				format!(
					"lowered module target {:?} does not match requested build target {target:?}",
					lowered.target
				),
			));
		}
		self.toolchain.verifier.verify()?;
		self.toolchain.llvm_codegen.verify()?;
		let workspace = BuildWorkspace::create(scratch_parent)?;
		let ir_path = workspace.path.join("kernel.ll");
		write_new(&ir_path, lowered.llvm_ir.as_bytes())?;
		let mut invocations = Vec::new();
		invoke(
			&self.toolchain.verifier,
			[
				OsString::from("-passes=verify"),
				OsString::from("-disable-output"),
				ir_path.as_os_str().to_owned(),
			],
			&workspace.path,
			&mut invocations,
		)?;

		let artifact = match target {
			KernelTarget::Amd(target) => self.build_hsaco(phase, lowered, target, &workspace, invocations)?,
			KernelTarget::Nvidia(target) => self.build_cubin(phase, lowered, *target, &workspace, invocations)?,
		};
		Ok(artifact)
	}

	fn build_hsaco(
		&self,
		phase: BuildPhase,
		lowered: &LoweredKernel,
		target: &AmdTarget,
		workspace: &BuildWorkspace,
		mut invocations: Vec<ToolInvocation>,
	) -> Result<BuiltArtifact, LoweringError> {
		let linker = self.toolchain.elf_linker.as_ref().ok_or_else(|| {
			LoweringError::new(
				LoweringErrorKind::InvalidTarget,
				"AMD artifact build requires a pinned ELF linker",
			)
		})?;
		linker.verify()?;
		let object = workspace.path.join("kernel.o");
		let output = workspace.path.join("kernel.hsaco");
		let (processor, features) = amd_processor_and_features(&target.target_id)?;
		let mut codegen_arguments = vec![
			OsString::from("-filetype=obj"),
			OsString::from("-march=amdgcn"),
			OsString::from(format!("-mcpu={processor}")),
			OsString::from(format!(
				"--amdhsa-code-object-version={}",
				target.code_object_version
			)),
		];
		if !features.is_empty() {
			codegen_arguments.push(OsString::from(format!("-mattr={features}")));
		}
		codegen_arguments.extend([
			workspace.path.join("kernel.ll").into_os_string(),
			OsString::from("-o"),
			object.as_os_str().to_owned(),
		]);
		invoke(
			&self.toolchain.llvm_codegen,
			codegen_arguments,
			&workspace.path,
			&mut invocations,
		)?;
		invoke(
			linker,
			[
				OsString::from("-shared"),
				OsString::from("--no-undefined"),
				object.into_os_string(),
				OsString::from("-o"),
				output.as_os_str().to_owned(),
			],
			&workspace.path,
			&mut invocations,
		)?;
		let bytes = read_regular(&output)?;
		let inspection = inspect_hsaco(
			&bytes,
			&target.target_id,
			target.code_object_version,
			&lowered.abi,
		)?;
		Ok(BuiltArtifact::Hsaco {
			bytes,
			inspection,
			provenance: self.provenance(phase, lowered, invocations),
		})
	}

	fn build_cubin(
		&self,
		phase: BuildPhase,
		lowered: &LoweredKernel,
		target: NvidiaTarget,
		workspace: &BuildWorkspace,
		mut invocations: Vec<ToolInvocation>,
	) -> Result<BuiltArtifact, LoweringError> {
		let assembler = self.toolchain.ptx_assembler.as_ref().ok_or_else(|| {
			LoweringError::new(
				LoweringErrorKind::InvalidTarget,
				"NVIDIA artifact build requires a pinned ptxas",
			)
		})?;
		assembler.verify()?;
		let ptx = workspace.path.join("kernel.ptx");
		let output = workspace.path.join("kernel.cubin");
		let architecture = target.architecture();
		invoke(
			&self.toolchain.llvm_codegen,
			[
				OsString::from("-march=nvptx64"),
				OsString::from(format!("-mcpu={architecture}")),
				OsString::from(format!("-mattr={}", target.llvm_ptx_feature())),
				workspace.path.join("kernel.ll").into_os_string(),
				OsString::from("-o"),
				ptx.as_os_str().to_owned(),
			],
			&workspace.path,
			&mut invocations,
		)?;
		invoke(
			assembler,
			[
				OsString::from(format!("-arch={architecture}")),
				ptx.as_os_str().to_owned(),
				OsString::from("-o"),
				output.as_os_str().to_owned(),
			],
			&workspace.path,
			&mut invocations,
		)?;
		let ptx = read_regular(&ptx)?;
		let bytes = read_regular(&output)?;
		let expected_sm = target
			.sm_major
			.checked_mul(10)
			.and_then(|value| value.checked_add(target.sm_minor))
			.ok_or_else(|| {
				LoweringError::new(
					LoweringErrorKind::ArithmeticOverflow,
					"NVIDIA SM identity overflowed",
				)
			})?;
		let inspection = inspect_cubin(&bytes, expected_sm, &lowered.abi.entry_symbol)?;
		Ok(BuiltArtifact::Cubin {
			ptx,
			bytes,
			inspection,
			provenance: self.provenance(phase, lowered, invocations),
		})
	}

	fn provenance(
		&self,
		phase: BuildPhase,
		lowered: &LoweredKernel,
		invocations: Vec<ToolInvocation>,
	) -> BuildProvenance {
		let mut tools = Vec::new();
		for invocation in &invocations {
			if tools.iter().any(|(path, _)| path == &invocation.program) {
				continue;
			}
			let digest = self
				.tool_digest(&invocation.program)
				.expect("every recorded invocation uses a pinned tool");
			tools.push((invocation.program.clone(), digest));
		}
		BuildProvenance {
			phase,
			llvm_ir: ArtifactDigest::of(lowered.llvm_ir.as_bytes()),
			tools,
			invocations,
		}
	}

	fn tool_digest(&self, path: &Path) -> Option<ArtifactDigest> {
		[
			Some(&self.toolchain.verifier),
			Some(&self.toolchain.llvm_codegen),
			self.toolchain.elf_linker.as_ref(),
			self.toolchain.ptx_assembler.as_ref(),
		]
		.into_iter()
		.flatten()
		.find(|tool| tool.path == path)
		.map(|tool| tool.digest)
	}
}

fn invoke(
	tool: &PinnedTool,
	arguments: impl IntoIterator<Item = OsString>,
	current_dir: &Path,
	invocations: &mut Vec<ToolInvocation>,
) -> Result<(), LoweringError> {
	tool.verify()?;
	let arguments = arguments.into_iter().collect::<Vec<_>>();
	let output = Command::new(&tool.path)
		.args(&arguments)
		.current_dir(current_dir)
		.env_clear()
		.env("LC_ALL", "C")
		.env("SOURCE_DATE_EPOCH", "0")
		.output()
		.map_err(|error| io_error("execute tool", &tool.path, error))?;
	invocations.push(ToolInvocation {
		program: tool.path.clone(),
		arguments: arguments
			.iter()
			.map(|argument| normalize_scratch_argument(argument, current_dir))
			.collect(),
	});
	if output.status.success() {
		return Ok(());
	}
	Err(LoweringError::new(
		LoweringErrorKind::ToolchainFailed,
		format!(
			"`{}` exited with {}: {}",
			tool.path.display(),
			output.status
				.code()
				.map_or_else(|| "signal".to_owned(), |code| code.to_string()),
			bounded_text(&output.stderr)
		),
	))
}

fn normalize_scratch_argument(argument: &std::ffi::OsStr, scratch: &Path) -> OsString {
	let path = Path::new(argument);
	path.strip_prefix(scratch).map_or_else(
		|_| argument.to_owned(),
		|relative| Path::new("@scratch").join(relative).into_os_string(),
	)
}

fn amd_processor_and_features(target_id: &str) -> Result<(&str, String), LoweringError> {
	let mut components = target_id.split(':');
	let processor = components.next().unwrap_or_default();
	if processor.is_empty() {
		return Err(LoweringError::new(
			LoweringErrorKind::InvalidTarget,
			"empty AMD processor target",
		));
	}
	let mut features = Vec::new();
	for feature in components {
		let (name, polarity) = feature.split_at(feature.len().saturating_sub(1));
		let prefix = match polarity {
			"+" => "+",
			"-" => "-",
			_ => {
				return Err(LoweringError::new(
					LoweringErrorKind::InvalidTarget,
					format!("AMD feature `{feature}` needs a + or - suffix"),
				));
			}
		};
		if name.is_empty()
			|| !name
				.bytes()
				.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
		{
			return Err(LoweringError::new(
				LoweringErrorKind::InvalidTarget,
				format!("AMD feature `{feature}` contains invalid characters"),
			));
		}
		features.push(format!("{prefix}{name}"));
	}
	Ok((processor, features.join(",")))
}

fn canonical_tool(path: &Path) -> Result<PathBuf, LoweringError> {
	if !path.is_absolute() {
		return Err(LoweringError::new(
			LoweringErrorKind::Io,
			format!("tool path `{}` is not absolute", path.display()),
		));
	}
	let canonical = fs::canonicalize(path).map_err(|error| io_error("canonicalize tool", path, error))?;
	let metadata = fs::metadata(&canonical).map_err(|error| io_error("inspect tool", &canonical, error))?;
	if !metadata.is_file() {
		return Err(LoweringError::new(
			LoweringErrorKind::Io,
			format!("tool `{}` is not a regular file", canonical.display()),
		));
	}
	Ok(canonical)
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<(), LoweringError> {
	let mut file = OpenOptions::new()
		.write(true)
		.create_new(true)
		.mode(0o600)
		.open(path)
		.map_err(|error| io_error("create build input", path, error))?;
	file.write_all(bytes)
		.map_err(|error| io_error("write build input", path, error))?;
	file.sync_all()
		.map_err(|error| io_error("sync build input", path, error))
}

fn read_regular(path: &Path) -> Result<Vec<u8>, LoweringError> {
	let metadata = fs::symlink_metadata(path).map_err(|error| io_error("inspect build output", path, error))?;
	if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
		return Err(LoweringError::new(
			LoweringErrorKind::Io,
			format!("build output `{}` is not a regular file", path.display()),
		));
	}
	fs::read(path).map_err(|error| io_error("read build output", path, error))
}

fn io_error(operation: &str, path: &Path, error: std::io::Error) -> LoweringError {
	LoweringError::new(
		LoweringErrorKind::Io,
		format!("{operation} `{}`: {error}", path.display()),
	)
}

fn bounded_text(bytes: &[u8]) -> String {
	let bytes = bytes.get(..ERROR_OUTPUT_LIMIT).unwrap_or(bytes);
	String::from_utf8_lossy(bytes).into_owned()
}

#[derive(Debug)]
struct BuildWorkspace {
	path: PathBuf,
}

impl BuildWorkspace {
	fn create(parent: &Path) -> Result<Self, LoweringError> {
		let metadata =
			fs::symlink_metadata(parent).map_err(|error| io_error("inspect scratch parent", parent, error))?;
		if !metadata.is_dir() || metadata.file_type().is_symlink() {
			return Err(LoweringError::new(
				LoweringErrorKind::Io,
				"scratch parent must be a real directory",
			));
		}
		if metadata.permissions().mode() & 0o077 != 0 {
			return Err(LoweringError::new(
				LoweringErrorKind::Io,
				format!(
					"scratch parent `{}` must not be accessible by group or others",
					parent.display()
				),
			));
		}
		for _ in 0..1024 {
			let nonce = WORKSPACE_NONCE.fetch_add(1, Ordering::Relaxed);
			let path = parent.join(format!("recipe-build-{}-{nonce}", std::process::id()));
			match fs::create_dir(&path) {
				Ok(()) => {
					fs::set_permissions(&path, fs::Permissions::from_mode(0o700))
						.map_err(|error| io_error("protect build workspace", &path, error))?;
					return Ok(Self { path });
				}
				Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
				Err(error) => return Err(io_error("create build workspace", &path, error)),
			}
		}
		Err(LoweringError::new(
			LoweringErrorKind::Io,
			"could not allocate a unique build workspace",
		))
	}
}

impl Drop for BuildWorkspace {
	fn drop(&mut self) {
		let _ = fs::remove_dir_all(&self.path);
	}
}

#[cfg(test)]
mod tests {
	use recipe_core::{
		AliasPermission, AliasRule, DType, ElementCount, IndexSpace, KernelInput, KernelInputId, KernelOutput,
		KernelOutputId, KernelTemplate, KernelTemplateId, ScalarInput, ScalarInstruction, ScalarOpcode,
		ScalarProgram, ScalarValueId, StaticBufferAccess,
	};

	use super::*;
	use crate::{LoweringOptions, lower_elementwise};

	#[test]
	fn parses_full_amd_target_features_without_shell_text() {
		assert_eq!(
			amd_processor_and_features("gfx1101:xnack-:sramecc+").unwrap(),
			("gfx1101", "-xnack,+sramecc".to_owned())
		);
		assert!(amd_processor_and_features("gfx1101:xnack").is_err());
	}

	#[test]
	fn tool_paths_must_be_absolute() {
		let error = PinnedTool::inspect(Path::new("llc")).unwrap_err();
		assert_eq!(error.kind, LoweringErrorKind::Io);
	}

	#[test]
	fn bounded_error_text_never_exceeds_limit() {
		let input = vec![b'x'; ERROR_OUTPUT_LIMIT + 100];
		assert_eq!(bounded_text(&input).len(), ERROR_OUTPUT_LIMIT);
	}

	#[test]
	fn provenance_paths_do_not_include_workspace_nonce() {
		let scratch = Path::new("/private/recipe-build-123-9");
		assert_eq!(
			normalize_scratch_argument(
				Path::new("/private/recipe-build-123-9/kernel.ll").as_os_str(),
				scratch,
			),
			OsString::from("@scratch/kernel.ll")
		);
		assert_eq!(
			normalize_scratch_argument(std::ffi::OsStr::new("-o"), scratch),
			OsString::from("-o")
		);
	}

	fn add_kernel(target: &KernelTarget) -> LoweredKernel {
		let left = ScalarValueId::new(1);
		let right = ScalarValueId::new(2);
		let result = ScalarValueId::new(3);
		let index_space = IndexSpace::new(vec![ElementCount::new(1_048_576).unwrap()]).unwrap();
		let access = StaticBufferAccess::contiguous(&index_space, DType::F32).unwrap();
		let template = KernelTemplate {
			id: KernelTemplateId::new(1),
			index_space,
			inputs: vec![
				KernelInput {
					id: KernelInputId::new(1),
					dtype: DType::F32,
					access: access.clone(),
				},
				KernelInput {
					id: KernelInputId::new(2),
					dtype: DType::F32,
					access: access.clone(),
				},
			],
			outputs: vec![KernelOutput {
				id: KernelOutputId::new(1),
				dtype: DType::F32,
				access,
			}],
			program: ScalarProgram {
				inputs: vec![
					ScalarInput {
						id: left,
						dtype: DType::F32,
					},
					ScalarInput {
						id: right,
						dtype: DType::F32,
					},
				],
				constants: Vec::new(),
				instructions: vec![ScalarInstruction {
					result,
					dtype: DType::F32,
					opcode: ScalarOpcode::Add,
					operands: vec![left, right],
				}],
				outputs: vec![result],
			},
			alias_rules: vec![
				AliasRule {
					input: KernelInputId::new(1),
					output: KernelOutputId::new(1),
					permission: AliasPermission::MayAliasExact,
				},
				AliasRule {
					input: KernelInputId::new(2),
					output: KernelOutputId::new(1),
					permission: AliasPermission::Forbidden,
				},
			],
		};
		lower_elementwise(
			&template,
			target,
			&LoweringOptions {
				entry_symbol: "recipe_add_f32".to_owned(),
				workgroup_lanes: 256,
			},
		)
		.unwrap()
	}

	struct PrivateScratch(PathBuf);

	impl PrivateScratch {
		fn create() -> Self {
			for nonce in 0..1024 {
				let path =
					std::env::temp_dir().join(format!("recipe-kernel-test-{}-{nonce}", std::process::id()));
				match fs::create_dir(&path) {
					Ok(()) => {
						fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
						return Self(path);
					}
					Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
					Err(error) => panic!("create private scratch: {error}"),
				}
			}
			panic!("could not allocate private test scratch directory");
		}
	}

	impl Drop for PrivateScratch {
		fn drop(&mut self) {
			let _ = fs::remove_dir(&self.0);
		}
	}

	#[test]
	#[ignore = "requires the host LLVM and AMDGPU toolchain"]
	fn builds_and_inspects_a_real_hsaco_deterministically() {
		let target = KernelTarget::Amd(AmdTarget {
			target_id: "gfx1101".to_owned(),
			code_object_version: 6,
		});
		let builder = ArtifactBuilder::new(OfflineToolchain {
			verifier: PinnedTool::inspect("/usr/bin/opt").unwrap(),
			llvm_codegen: PinnedTool::inspect("/usr/bin/llc").unwrap(),
			elf_linker: Some(PinnedTool::inspect("/usr/bin/ld.lld").unwrap()),
			ptx_assembler: None,
		})
		.unwrap();
		let scratch = PrivateScratch::create();
		let first = builder
			.build(
				BuildPhase::Offline,
				&add_kernel(&target),
				&target,
				&scratch.0,
			)
			.unwrap();
		let second = builder
			.build(
				BuildPhase::Offline,
				&add_kernel(&target),
				&target,
				&scratch.0,
			)
			.unwrap();
		assert_eq!(first, second);
	}

	#[test]
	#[ignore = "requires the host LLVM and CUDA ptxas toolchain"]
	fn builds_and_inspects_a_real_cubin_deterministically() {
		let target = KernelTarget::Nvidia(NvidiaTarget {
			sm_major: 8,
			sm_minor: 6,
			ptx_isa: 75,
		});
		let builder = ArtifactBuilder::new(OfflineToolchain {
			verifier: PinnedTool::inspect("/usr/bin/opt").unwrap(),
			llvm_codegen: PinnedTool::inspect("/usr/bin/llc").unwrap(),
			elf_linker: None,
			ptx_assembler: Some(PinnedTool::inspect("/opt/cuda/bin/ptxas").unwrap()),
		})
		.unwrap();
		let scratch = PrivateScratch::create();
		let first = builder
			.build(
				BuildPhase::Offline,
				&add_kernel(&target),
				&target,
				&scratch.0,
			)
			.unwrap();
		let second = builder
			.build(
				BuildPhase::Offline,
				&add_kernel(&target),
				&target,
				&scratch.0,
			)
			.unwrap();
		assert_eq!(first, second);
	}
}
