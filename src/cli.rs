use std::collections::VecDeque; use std::env; use std::ffi::{OsStr, OsString}; use std::fmt::Write as _;
use std::fs::{self, DirBuilder, File, OpenOptions}; use std::io::{BufReader, BufWriter, Read as _, Write as _};
use std::os::unix::ffi::{OsStrExt as _, OsStringExt as _};
use std::os::unix::fs::{DirBuilderExt as _, MetadataExt as _, OpenOptionsExt as _, PermissionsExt as _};
use std::os::unix::process::ExitStatusExt as _; use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Output, Stdio}; use std::sync::atomic::{AtomicU64, Ordering};
use std::thread::JoinHandle;

use recipe_core::{Digest, Label}; use recipe_ingest::{
	GgufLimits, gguf_to_structural_ogdl_stream, structural_ogdl_declared_gguf_bytes, structural_ogdl_to_gguf_stream, };
use recipe_kernel::{ArtifactDigest, OfflineToolchain, PinnedTool}; use recipe_native_probe::{
	BackendLibrary, CudaProbeConfig, HsaProbeConfig, KernelBuildConfig, NativeGpuProbe, NativeProbeConfig, };
use recipe_probe::local::{LocalHostBenchmarks, LocalSystemDiscovery}; use recipe_probe::{
	CacheIdentity, ExplicitPathProfileCache, HostDiscovery as _, HostInventory, MeasuredProfile, ProbeEngine, SeedContract,
};

const ACTIVE_NATIVE_RECEIPT: &str = "active-native-v1";
const ACTIVE_NATIVE_MAGIC: &str = "recipe-active-native-v1";
const ACTIVE_NATIVE_MAXIMUM_BYTES: usize = 64 * 1024; const RUN_OUTPUT_TAIL_BYTES: usize = 64 * 1024;
const RUN_OUTPUT_BUFFER_BYTES: usize = 8 * 1024; const PRIVATE_FILE_MODE: u32 = 0o600;
const PRIVATE_DIRECTORY_MASK: u32 = 0o077; const O_CLOEXEC: i32 = 0o2_000_000; const O_NOFOLLOW: i32 = 0o400_000;
const CUDA_BACKEND: &str = "nvidia-cuda-driver";
const HSA_BACKEND: &str = "amd-rocr-hsa";
const RECEIPT_FIELDS: [&str; 16] = [
	"profile",
	"profile_schema",
	"profile_digest",
	"host_memory_key",
	"pci_sysfs_root",
	"scratch_parent",
	"cuda_library",
	"hsa_library",
	"llvm_opt",
	"llvm_llc",
	"lld",
	"ptxas",
	"ptx_isa",
	"hsa_code_object_version",
	"release",
	"fma_chain_length",
];

const USAGE: &str = "\
Usage:
	recipe run FILE.rs	[ARGS...]
	recipe probe		[OPTIONS]
	recipe convert INPUT OUTPUT
";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct ProbeOptions { contract: Option<PathBuf>, profile: Option<PathBuf>, cuda_libraries: Vec<PathBuf>,
	hsa_libraries: Vec<PathBuf>, llvm_opt: Option<PathBuf>, llvm_llc: Option<PathBuf>, lld: Option<PathBuf>,
	ptxas: Option<PathBuf>, }

#[derive(Debug)]
pub(crate) struct CurrentNativeInputs { pub config: NativeProbeConfig, pub host: HostInventory,
	pub profile_identity: CacheIdentity, pub profile_path: PathBuf, pub native: NativeGpuProbe, }

#[derive(Clone, Debug, PartialEq, Eq)]
struct PinnedNativeFile { path: PathBuf, digest: [u8; 32], }

/// Canonical v1 runtime handoff: fixed-order tab records with byte-hex paths
/// and labels, lowercase SHA-256 identities, and decimal scalar settings.
#[derive(Clone, Debug, PartialEq, Eq)]
struct ActiveNativeReceipt { profile_path: PathBuf, profile_identity: CacheIdentity, host_memory_key: Label,
	pci_sysfs_root: PathBuf, scratch_parent: PathBuf, cuda_library: Option<PinnedNativeFile>,
	hsa_library: Option<PinnedNativeFile>, verifier: PinnedNativeFile, llvm_codegen: PinnedNativeFile,
	elf_linker: Option<PinnedNativeFile>, ptx_assembler: Option<PinnedNativeFile>, ptx_isa: u16,
	hsa_code_object_version: u8, release: Label, fma_chain_length: u16, }

pub fn main() -> ExitCode { match run(env::args_os().skip(1).collect()) { Ok(()) => ExitCode::SUCCESS, Err(error) => {
			eprintln!("recipe: {error}");
			ExitCode::FAILURE } } }

fn run(arguments: Vec<OsString>) -> Result<(), String> { let Some(command) = arguments.first() else {
		return Err(format!("missing command\n\n{USAGE}"));
	};
	if command == "-h" || command == "--help" {
		print!("{USAGE}");
		return Ok(()); }
	match command.to_str() {
		Some("run") => {
			let Some(path) = arguments.get(1) else { return Err(format!(
					"`recipe run` requires a Rust source file\n\n{USAGE}"
				)); };
			if path == "-h" || path == "--help" {
				print!("{USAGE}");
				return Ok(()); }
			run_source(path, &arguments[2..]) }
		Some("probe") => {
			if arguments.len() == 2 && (arguments[1] == "-h" || arguments[1] == "--help") {
				print!("{USAGE}");
				return Ok(()); }
			let options = parse_probe_options(&arguments[1..])?; run_probe(options) }
		Some("convert") => {
			if arguments.len() == 2 && (arguments[1] == "-h" || arguments[1] == "--help") {
				print!("{USAGE}");
				return Ok(()); }
			run_model_conversion(&arguments[1..]) }
		Some(_) => Err(format!(
			"unknown command {:?}; expected `run`, `probe`, or `convert`\n\n{USAGE}",
			command )),
		None => Err("commands must be valid UTF-8".to_owned()),
	} }

fn run_model_conversion(arguments: &[OsString]) -> Result<(), String> { if arguments.len() != 2 { return Err(format!(
			"`recipe convert` requires exactly an input and output path\n\n{USAGE}"
		)); }
	let input = Path::new(&arguments[0]); let output = Path::new(&arguments[1]); let input_extension = input.extension();
	let output_extension = output.extension(); match (input_extension, output_extension) {
		(Some(input_extension), Some(output_extension))
			if input_extension == OsStr::new("gguf") && output_extension == OsStr::new("ogdl") =>
		{ let source = File::open(input)
				.map_err(|error| format!("open conversion input {}: {error}", input.display()))?;
			let source_bytes = source .metadata()
				.map_err(|error| format!("inspect conversion input {}: {error}", input.display()))?
				.len(); let limits = conversion_limits(source_bytes, source_bytes)?; let mut source = BufReader::new(source);
			write_new_conversion_output(output, |destination| { let mut destination = BufWriter::new(destination);
				gguf_to_structural_ogdl_stream(&mut source, &mut destination, limits)
					.map_err(|error| format!("convert {} to structural OGDL: {error}", input.display()))?;
				destination .flush()
					.map_err(|error| format!("flush conversion output {}: {error}", output.display()))
			})?; }
		(Some(input_extension), Some(output_extension))
			if input_extension == OsStr::new("ogdl") && output_extension == OsStr::new("gguf") =>
		{ let source = File::open(input)
				.map_err(|error| format!("open conversion input {}: {error}", input.display()))?;
			let source_bytes = source .metadata()
				.map_err(|error| format!("inspect conversion input {}: {error}", input.display()))?
				.len(); let mut source = BufReader::new(source); let output_bytes = structural_ogdl_declared_gguf_bytes(&mut source)
				.map_err(|error| format!("inspect structural OGDL {}: {error}", input.display()))?;
			let limits = conversion_limits(source_bytes, output_bytes)?; write_new_conversion_output(output, |destination| {
				structural_ogdl_to_gguf_stream(&mut source, destination, limits)
					.map_err(|error| format!("convert {} to GGUF: {error}", input.display()))
			})?; }
		_ => { return Err(format!(
				"`recipe convert` requires `.gguf .ogdl` or `.ogdl .gguf`, found {} -> {}",
				input.display(), output.display() )); } }
	println!("Converted {} -> {}", input.display(), output.display());
	Ok(()) }

fn conversion_limits(source_bytes: u64, output_bytes: u64) -> Result<GgufLimits, String> {
	let source_bound = source_bytes.max(1); GgufLimits::new( output_bytes.max(1), source_bound, source_bound, 4,
		source_bound, source_bound, source_bound, )
		.map_err(|error| format!("construct conversion bounds: {error}"))
}

fn write_new_conversion_output( path: &Path, write: impl FnOnce(&mut File) -> Result<(), String>,
) -> Result<(), String> { let mut file = OpenOptions::new() .read(true) .write(true) .create_new(true)
		.mode(PRIVATE_FILE_MODE) .open(path)
		.map_err(|error| format!("create conversion output {}: {error}", path.display()))?;
	if let Err(error) = write(&mut file).and_then(|()| { file.sync_all()
			.map_err(|error| format!("sync conversion output {}: {error}", path.display()))
	}) { drop(file); let cleanup = fs::remove_file(path); return match cleanup { Ok(()) => Err(error),
			Err(cleanup) => Err(format!(
				"{error}; remove partial output {}: {cleanup}",
				path.display(), )), }; }
	Ok(()) }

fn run_source(path: &OsStr, arguments: &[OsString]) -> Result<(), String> { let requested_source = Path::new(path);
	let source = fs::canonicalize(requested_source).map_err(|error| { format!(
			"read training source {}: {error}",
			requested_source.display() ) })?; let metadata = fs::metadata(&source).map_err(|error| { format!(
			"read training source {}: {error}",
			requested_source.display() ) })?; if !metadata.is_file() { return Err(format!(
			"training source {} is not a regular file",
			requested_source.display() )); }
	let source_directory = source.parent().ok_or_else(|| { format!(
			"training source {} has no parent directory",
			source.display() ) })?; let source_text = fs::read_to_string(&source)
		.map_err(|error| format!("read training source {}: {error}", source.display()))?;

	let (library_root, library) = locate_recipe_library()?; let state_root = private_state_root()?;
	let run_root = state_root.join("run");
	ensure_private_directory(&run_root)?; static NEXT_RUN_SOURCE: AtomicU64 = AtomicU64::new(1);
	let sequence = NEXT_RUN_SOURCE.fetch_add(1, Ordering::Relaxed);
	let binary = run_root.join(format!("recipe-run-{}-{sequence}", std::process::id()));
	let mut recipe_external = OsString::from("recipe=");
	recipe_external.push(library.as_os_str());

	let lowering = crate::source_frontend::lower_recipe_source(&source, &source_text)?; let transformed_source = lowering
		.as_ref() .map(|rewrite| TransformedRunSource::create(&source, sequence, rewrite.generated())) .transpose()?;
	let compiler_source = transformed_source .as_ref() .map_or(source.as_path(), |transformed| transformed.path());
	let remap = transformed_source .as_ref() .map(|transformed| (source.as_path(), transformed.path()));
	let compilation = compile_run_source( compiler_source, &binary, &library_root, &recipe_external, remap, )?;
	let diagnostics = crate::source_frontend::DiagnosticStream::parse(&compilation.stderr);
	let rendering = match (&lowering, &transformed_source) {
		(Some(rewrite), Some(transformed)) => diagnostics.mapped_rendering(rewrite, transformed.path(), &source),
		_ => diagnostics.original_rendering(), }; emit_compiler_output(&compilation.stdout, &rendering)?;
	if !compilation.status.success() { let _ = fs::remove_file(&binary); return Err(format!(
			"rustc failed for {} with {}",
			source.display(), compilation.status )); }

	let run_output = run_compiled_source_live(&binary, arguments, source_directory, &source);
	let cleanup = fs::remove_file(&binary).map_err(|error| { format!(
			"remove compiled training binary {}: {error}",
			binary.display() ) }); let run_output = run_output?; cleanup?; if !run_output.status.success() {
		return Err(run_status_message( &source, &run_output.status, &run_output.stdout, &run_output.stderr, )); }
	Ok(()) }

#[derive(Debug)]
struct LiveRunOutput { status: std::process::ExitStatus, stdout: CapturedOutputTail, stderr: CapturedOutputTail, }

#[derive(Debug)]
struct CapturedOutputTail { bytes: Vec<u8>, truncated: bool, }

#[derive(Debug)]
struct OutputTailBuffer { bytes: VecDeque<u8>, truncated: bool, }

impl OutputTailBuffer { fn new() -> Self { Self { bytes: VecDeque::with_capacity(RUN_OUTPUT_TAIL_BYTES),
			truncated: false, } }

	fn extend(&mut self, chunk: &[u8]) { if chunk.len() >= RUN_OUTPUT_TAIL_BYTES { self.bytes.clear(); self.bytes.extend(
				chunk[chunk.len().saturating_sub(RUN_OUTPUT_TAIL_BYTES)..] .iter() .copied(), ); self.truncated = true; return; }
		self.bytes.extend(chunk.iter().copied()); let overflow = self.bytes.len().saturating_sub(RUN_OUTPUT_TAIL_BYTES);
		if overflow != 0 { self.bytes.drain(..overflow); self.truncated = true; } }

	fn finish(self) -> CapturedOutputTail { CapturedOutputTail { bytes: self.bytes.into(), truncated: self.truncated, } } }

#[derive(Clone, Copy, Debug)]
enum RunOutputDestination { Stdout, Stderr, }

impl RunOutputDestination {
	const fn name(self) -> &'static str {
		match self {
			Self::Stdout => "stdout",
			Self::Stderr => "stderr",
		} } }

fn run_compiled_source_live( command: &Path, arguments: &[OsString], current_directory: &Path, source: &Path,
) -> Result<LiveRunOutput, String> { let interrupt = crate::signal::SigintGuard::install()
		.map_err(|error| format!("install recipe run SIGINT handler: {error}"))?;
	let mut child = Command::new(command) .args(arguments) .current_dir(current_directory) .stdout(Stdio::piped())
		.stderr(Stdio::piped()) .spawn()
		.map_err(|error| format!("run {}: {error}", source.display()))?;
	let stdout = child .stdout .take()
		.ok_or_else(|| format!("capture training stdout for {}", source.display()))?;
	let stderr = child .stderr .take()
		.ok_or_else(|| format!("capture training stderr for {}", source.display()))?;
	let stdout_forwarder = match spawn_run_output_forwarder(stdout, RunOutputDestination::Stdout) {
		Ok(forwarder) => forwarder, Err(error) => { let _ = child.kill(); let _ = child.wait();
			return Err(format!("start training stdout forwarder: {error}"));
		} }; let stderr_forwarder = match spawn_run_output_forwarder(stderr, RunOutputDestination::Stderr) {
		Ok(forwarder) => forwarder, Err(error) => { let _ = child.kill(); let _ = child.wait();
			let _ = stdout_forwarder.join();
			return Err(format!("start training stderr forwarder: {error}"));
		} };

	let mut forwarded_interrupt = false; let status = loop { if interrupt.requested() && !forwarded_interrupt {
			forwarded_interrupt = true; match crate::signal::send_sigint(child.id()) { Ok(()) => {}
				Err(error) if error.raw_os_error() == Some(libc::ESRCH) => {}
				Err(error) => { let _ = child.kill(); let _ = child.wait(); break Err(format!(
						"forward SIGINT to training child {}: {error}",
						child.id() )); } } }
		match child.try_wait() { Ok(Some(status)) => break Ok(status),
			Ok(None) => std::thread::sleep(std::time::Duration::from_millis(10)), Err(error) => { let _ = child.kill();
				let _ = child.wait();
				break Err(format!("wait for {}: {error}", source.display()));
			} } }; let stdout = join_run_output_forwarder(stdout_forwarder, RunOutputDestination::Stdout);
	let stderr = join_run_output_forwarder(stderr_forwarder, RunOutputDestination::Stderr); Ok(LiveRunOutput {
		status: status?, stdout: stdout?, stderr: stderr?, }) }

fn spawn_run_output_forwarder<R>( reader: R, destination: RunOutputDestination,
) -> std::io::Result<JoinHandle<Result<CapturedOutputTail, String>>>
where
	R: std::io::Read + Send + 'static,
{ std::thread::Builder::new()
		.name(format!("recipe-run-{}", destination.name()))
		.spawn(move || match destination { RunOutputDestination::Stdout => { let output = std::io::stdout();
				forward_run_output_stream(reader, output.lock(), destination) }
			RunOutputDestination::Stderr => { let output = std::io::stderr();
				forward_run_output_stream(reader, output.lock(), destination) } }) }

fn forward_run_output_stream<R: std::io::Read, W: std::io::Write>( mut reader: R, mut writer: W,
	destination: RunOutputDestination, ) -> Result<CapturedOutputTail, String> {
	let mut captured = OutputTailBuffer::new(); let mut buffer = [0_u8; RUN_OUTPUT_BUFFER_BYTES]; loop { let count = reader
			.read(&mut buffer)
			.map_err(|error| format!("read training {}: {error}", destination.name()))?;
		if count == 0 { break; }
		captured.extend(&buffer[..count]); writer.write_all(&buffer[..count])
			.map_err(|error| format!("write training {}: {error}", destination.name()))?;
		writer.flush()
			.map_err(|error| format!("flush training {}: {error}", destination.name()))?;
	}
	Ok(captured.finish()) }

fn join_run_output_forwarder( forwarder: JoinHandle<Result<CapturedOutputTail, String>>,
	destination: RunOutputDestination, ) -> Result<CapturedOutputTail, String> { forwarder.join().map_err(|panic| {
		format!(
			"training {} forwarder panicked: {}",
			destination.name(), thread_panic_detail(panic) ) })? }

fn thread_panic_detail(panic: Box<dyn std::any::Any + Send>) -> String {
	if let Some(message) = panic.downcast_ref::<&str>() { (*message).to_owned()
	} else if let Some(message) = panic.downcast_ref::<String>() { message.clone() } else {
		"non-string panic payload".to_owned()
	} }

fn run_status_message( source: &Path, status: &std::process::ExitStatus, stdout: &CapturedOutputTail,
	stderr: &CapturedOutputTail, ) -> String { let reason = if let Some(bytes) = allocation_bytes(&stderr.bytes) { format!(
			"out of memory while requesting {bytes} bytes ({:.3} GB)",
			bytes_to_gb(bytes) ) } else if let Some(signal) = status.signal() {
		format!("terminated by signal {signal}")
	} else if let Some(code) = status.code() {
		format!("exited with code {code}")
	} else {
		format!("exited with {status}")
	}; let truncated = match (stdout.truncated, stderr.truncated) {
		(true, true) => "; captured stdout and stderr tails were truncated",
		(true, false) => "; captured stdout tail was truncated",
		(false, true) => "; captured stderr tail was truncated",
		(false, false) => "",
	};
	format!("training source {} failed: {reason}{truncated}", source.display())
}

fn allocation_bytes(stderr: &[u8]) -> Option<u64> { let stderr = std::str::from_utf8(stderr).ok()?;
	let marker = "memory allocation of ";
	let start = stderr.find(marker)? + marker.len(); let rest = &stderr[start..];
	let bytes_end = rest.char_indices().find(|(_, ch)| !ch.is_ascii_digit())?.0; rest[..bytes_end].parse().ok() }

fn bytes_to_gb(bytes: u64) -> f64 { (bytes as f64) / 1_000_000_000.0 }

fn compile_run_source( source: &Path, binary: &Path, library_root: &Path, recipe_external: &OsStr,
	remap: Option<(&Path, &Path)>, ) -> Result<Output, String> {
	let compiler_path = env::var_os("RUSTC").unwrap_or_else(|| OsString::from("rustc"));
	let mut compiler = Command::new(compiler_path); compiler .arg(source) .args([
			"--crate-name",
			"recipe_run",
			"--crate-type",
			"bin",
			"--edition",
			"2024",
			"-Dunused_must_use",
			"--error-format=json",
		])
		.arg("-L")
		.arg(library_root);
	let dependencies = library_root.join("deps");
	if dependencies.is_dir() {
		compiler.arg("-L").arg(&dependencies);
	}
	for dependency in cargo_build_output_roots(library_root)? {
		compiler.arg("-L").arg(dependency);
	}
	if let Some((original, transformed)) = remap { let mut mapping = transformed.as_os_str().to_owned();
		mapping.push("=");
		mapping.push(original.as_os_str());
		compiler.arg("--remap-path-prefix").arg(mapping);
	}
	compiler
		.arg("--extern")
		.arg(recipe_external)
		.arg("-o")
		.arg(binary) .output()
		.map_err(|error| format!("start rustc for {}: {error}", source.display()))
}

fn emit_compiler_output(stdout: &[u8], diagnostics: &str) -> Result<(), String> { std::io::stdout() .write_all(stdout)
		.map_err(|error| format!("write rustc standard output: {error}"))?;
	std::io::stderr() .write_all(diagnostics.as_bytes())
		.map_err(|error| format!("write rustc diagnostics: {error}"))
}

struct TransformedRunSource { path: PathBuf, }

impl TransformedRunSource { fn create(source: &Path, sequence: u64, contents: &str) -> Result<Self, String> {
		let parent = source.parent().ok_or_else(|| { format!(
				"training source {} has no parent directory",
				source.display() ) })?; let file_name = source .file_name()
			.ok_or_else(|| format!("training source {} has no file name", source.display()))?;
		let mut temporary_name = OsString::from(".");
		temporary_name.push(file_name);
		temporary_name.push(format!(".recipe-{}-{sequence}.rs", std::process::id()));
		let path = parent.join(temporary_name); let mut file = OpenOptions::new() .write(true) .create_new(true)
			.mode(PRIVATE_FILE_MODE) .custom_flags(O_CLOEXEC | O_NOFOLLOW) .open(&path) .map_err(|error| { format!(
					"create transformed Recipe source {}: {error}",
					path.display() ) })?; if let Err(error) = file.write_all(contents.as_bytes()) { let _ = fs::remove_file(&path);
			return Err(format!(
				"write transformed Recipe source {}: {error}",
				path.display() )); }
		Ok(Self { path }) }

	fn path(&self) -> &Path { &self.path } }

impl Drop for TransformedRunSource { fn drop(&mut self) { let _ = fs::remove_file(&self.path); } }

fn locate_recipe_library() -> Result<(PathBuf, PathBuf), String> {
	let executable = env::current_exe().map_err(|error| format!("locate recipe executable: {error}"))?;
	let mut roots = Vec::new(); if let Some(parent) = executable.parent() { roots.push(parent.to_path_buf());
		if parent.file_name() == Some(OsStr::new("deps"))
			&& let Some(profile) = parent.parent()
		{ roots.push(profile.to_path_buf()); } }
	if let Ok(current) = env::current_dir() {
		roots.push(current.join("target/debug"));
		roots.push(current.join("target/release"));
	}
	roots.push(PathBuf::from("/usr/lib/recipe"));
	for root in roots { if let Some(library) = newest_recipe_library(&root) { return Ok((root, library)); } }
	Err(
		"librecipe.rlib is unavailable; build Recipe first or install its source-runner libraries under /usr/lib/recipe"
			.to_owned(), ) }

fn newest_recipe_library(root: &Path) -> Option<PathBuf> {
	let dependencies = root.join("deps");
	let newest = fs::read_dir(&dependencies).ok().and_then(|entries| { entries .filter_map(Result::ok)
			.filter_map(|entry| { let name = entry.file_name(); let name = name.as_bytes();
				if !name.starts_with(b"librecipe-") || !name.ends_with(b".rlib") {
					return None; }
				let metadata = entry.metadata().ok()?; if !metadata.is_file() { return None; }
				Some((metadata.modified().ok()?, entry.path())) }) .max_by_key(|(modified, _)| *modified) .map(|(_, path)| path) });
	newest .or_else(|| newest_cargo_build_output_library(root)) .or_else(|| {
		let library = root.join("librecipe.rlib");
		library.is_file().then_some(library) }) }

fn newest_cargo_build_output_library(root: &Path) -> Option<PathBuf> {
	let recipe = root.join("build/recipe");
	fs::read_dir(recipe) .ok()? .filter_map(Result::ok)
		.filter_map(|entry| fs::read_dir(entry.path().join("out")).ok())
		.flatten() .filter_map(Result::ok) .filter_map(|entry| { let name = entry.file_name(); let name = name.as_bytes();
			if !name.starts_with(b"librecipe-") || !name.ends_with(b".rlib") {
				return None; }
			let metadata = entry.metadata().ok()?; if !metadata.is_file() { return None; }
			Some((metadata.modified().ok()?, entry.path())) }) .max_by_key(|(modified, _)| *modified) .map(|(_, path)| path) }

fn cargo_build_output_roots(root: &Path) -> Result<Vec<PathBuf>, String> {
	const MAXIMUM_BUILD_OUTPUT_ROOTS: usize = 4096;
	let build = root.join("build");
	let packages = match fs::read_dir(&build) { Ok(packages) => packages,
		Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
		Err(error) => return Err(format!("read Cargo build outputs {}: {error}", build.display())),
	}; let mut roots = Vec::new(); for package in packages {
		let package = package.map_err(|error| format!("read Cargo package output: {error}"))?;
		if !package .file_type()
			.map_err(|error| format!("inspect Cargo package output {}: {error}", package.path().display()))?
			.is_dir()
		{ continue; }
		let variants = fs::read_dir(package.path())
			.map_err(|error| format!("read Cargo package variants {}: {error}", package.path().display()))?;
		for variant in variants { let output = variant
				.map_err(|error| format!("read Cargo package variant: {error}"))?
				.path()
				.join("out");
			if output.is_dir() { if roots.len() == MAXIMUM_BUILD_OUTPUT_ROOTS { return Err(format!(
						"Cargo build output search exceeds {MAXIMUM_BUILD_OUTPUT_ROOTS} directories"
					)); }
				roots.push(output); } } }
	roots.sort(); Ok(roots) }

fn parse_probe_options(arguments: &[OsString]) -> Result<ProbeOptions, String> {
	let mut options = ProbeOptions::default(); let mut index = 0; while index < arguments.len() {
		let option = arguments[index] .to_str()
			.ok_or_else(|| "command-line options must be valid UTF-8".to_owned())?;
		let value = arguments .get(index + 1)
			.ok_or_else(|| format!("option {option} requires a path argument"))?;
		let path = PathBuf::from(value); match option {
			"--contract" => set_once(&mut options.contract, path, option)?,
			"--profile" => set_once(&mut options.profile, path, option)?,
			"--cuda-driver" => options.cuda_libraries.push(path),
			"--hsa-runtime" => options.hsa_libraries.push(path),
			"--llvm-opt" => set_once(&mut options.llvm_opt, path, option)?,
			"--llvm-llc" => set_once(&mut options.llvm_llc, path, option)?,
			"--lld" => set_once(&mut options.lld, path, option)?,
			"--ptxas" => set_once(&mut options.ptxas, path, option)?,
			_ => return Err(format!("unknown probe option {option:?}\n\n{USAGE}")),
		}
		index += 2; }
	Ok(options) }

fn set_once(slot: &mut Option<PathBuf>, value: PathBuf, option: &str) -> Result<(), String> {
	if slot.replace(value).is_some() {
		Err(format!("{option} may be supplied only once"))
	} else { Ok(()) } }

fn run_probe(options: ProbeOptions) -> Result<(), String> { require_bare_metal()?; let seed = match &options.contract {
		Some(path) => SeedContract::read(path).map_err(|error| error.to_string())?, None => {
			SeedContract::parse(include_str!("../topology/contract.toml")).map_err(|error| error.to_string())?
		} };

	let state_root = private_state_root()?;
	let scratch = state_root.join("scratch");
	ensure_private_directory(&scratch)?;

	let host = LocalSystemDiscovery::with_benchmark_roots(vec![state_root.clone()]);
	let inventory = host.discover_host().map_err(|error| error.to_string())?; let host_memory_key = inventory .ram .first()
		.map(|domain| domain.key.clone())
		.ok_or_else(|| "host discovery returned no RAM domain".to_owned())?;
	let config = native_config(&options, host_memory_key, scratch)?;
	let native = NativeGpuProbe::new(config.clone()).map_err(|error| error.to_string())?;
	let host_benchmarks = LocalHostBenchmarks; let engine = ProbeEngine::new(&host, &native, &host_benchmarks, &native);
	let peers = []; let identity = engine .current_cache_identity(&seed, &peers) .map_err(|error| error.to_string())?;

	let profile_path = match options.profile.as_ref() { Some(path) => path.clone(), None => {
			let directory = state_root.join("profiles");
			ensure_private_directory(&directory)?; directory.join(format!(
				"measured-v{}-{}.recipe-profile",
				identity.schema, hex(identity.digest) )) } };
	let cache = ExplicitPathProfileCache::new(&profile_path).map_err(|error| error.to_string())?;
	let was_cached = profile_path.exists(); let profile = engine .load_or_probe_and_store(&seed, &peers, &cache)
		.map_err(|error| error.to_string())?; let receipt = ActiveNativeReceipt::capture(&profile_path, &profile, &config)?;
	write_active_native_receipt(&state_root, &receipt)?;

	println!("profile={}", profile_path.display());
	println!(
		"source={}",
		if was_cached {
			"validated-cache"
		} else {
			"fresh-measurement"
		} );
	println!("cache_identity={}", hex(profile.cache_identity.digest));
	println!(
		"topology_identity={}",
		hex(profile.topology.identity.digest()) ); println!(
		"discovery_identity={}",
		hex(profile.discovery.identity.digest()) );
	println!("machines={}", profile.topology.machines.len());
	println!("devices={}", profile.topology.devices.len());
	println!("directed_links={}", profile.topology.links.len());
	Ok(()) }

pub(crate) fn current_native_inputs() -> Result<CurrentNativeInputs, String> { require_bare_metal()?;
	let state_root = private_state_root()?;
	let host_discovery = LocalSystemDiscovery::with_benchmark_roots(vec![state_root.clone()]); let host = host_discovery
		.discover_host() .map_err(|error| error.to_string())?;
	let (config, profile_path, profile_identity) = match load_active_native_receipt(&state_root)? { Some(receipt) => {
			if !host .ram .iter() .any(|domain| domain.key == receipt.host_memory_key)
			{ return Err(format!(
					"active native receipt RAM origin {} is absent from current host discovery; rerun `recipe probe`",
					receipt.host_memory_key )); }
			let profile_path = receipt.profile_path.clone(); let profile_identity = receipt.profile_identity;
			let config = receipt.reopen_config()?; (config, profile_path, profile_identity) }
		None => { let options = ProbeOptions::default();
			let seed = SeedContract::parse(include_str!("../topology/contract.toml"))
				.map_err(|error| error.to_string())?;
			let scratch = state_root.join("scratch");
			ensure_private_directory(&scratch)?; let host_memory_key = host .ram .first() .map(|domain| domain.key.clone())
				.ok_or_else(|| "host discovery returned no RAM domain".to_owned())?;
			let config = native_config(&options, host_memory_key, scratch)?;
			let native = NativeGpuProbe::new(config.clone()).map_err(|error| error.to_string())?;
			let host_benchmarks = LocalHostBenchmarks;
			let engine = ProbeEngine::new(&host_discovery, &native, &host_benchmarks, &native); let profile_identity = engine
				.current_cache_identity(&seed, &[]) .map_err(|error| error.to_string())?;
			let profiles = state_root.join("profiles");
			ensure_private_directory(&profiles)?; let profile_path = profiles.join(format!(
				"measured-v{}-{}.recipe-profile",
				profile_identity.schema, hex(profile_identity.digest) )); (config, profile_path, profile_identity) } };
	let native = NativeGpuProbe::new(config.clone()).map_err(|error| error.to_string())?; Ok(CurrentNativeInputs { config,
		host, profile_identity, profile_path, native, }) }

impl ActiveNativeReceipt {
	fn capture(profile_path: &Path, profile: &MeasuredProfile, config: &NativeProbeConfig) -> Result<Self, String> {
		let profile_path = inspect_private_regular_path(profile_path, "measured profile")?;
		let pci_sysfs_root = inspect_canonical_directory(&config.pci_sysfs_root, "PCI sysfs root")?;
		let scratch_parent = inspect_private_directory(&config.kernels.scratch_parent, "kernel scratch directory")?;
		for backend in profile.discovery.devices.iter().filter_map(|device| { device.calculation .as_ref()
				.map(|calculation| calculation.target.backend.as_str()) }) { if !matches!(backend, CUDA_BACKEND | HSA_BACKEND) {
				return Err(format!(
					"measured profile contains unsupported native backend {backend}; no v1 receipt identity exists"
				)); } }
		let cuda_required = profile_uses_backend(profile, CUDA_BACKEND);
		let hsa_required = profile_uses_backend(profile, HSA_BACKEND); let cuda_library = if cuda_required {
			Some(required_selected_library( &config.cuda.library,
				"CUDA Driver",
			)?) } else { None }; let hsa_library = if hsa_required { Some(required_selected_library( &config.hsa.library,
				"ROCr/HSA runtime",
			)?) } else { None }; Ok(Self { profile_path, profile_identity: profile.cache_identity,
			host_memory_key: config.host_memory_key.clone(), pci_sysfs_root, scratch_parent, cuda_library, hsa_library,
			verifier: PinnedNativeFile::from_tool(&config.kernels.toolchain.verifier, "LLVM opt")?,
			llvm_codegen: PinnedNativeFile::from_tool(&config.kernels.toolchain.llvm_codegen, "LLVM llc")?,
			elf_linker: config .kernels .toolchain .elf_linker .as_ref()
				.map(|tool| PinnedNativeFile::from_tool(tool, "LLVM lld"))
				.transpose()?, ptx_assembler: config .kernels .toolchain .ptx_assembler .as_ref()
				.map(|tool| PinnedNativeFile::from_tool(tool, "NVIDIA ptxas"))
				.transpose()?, ptx_isa: config.cuda.ptx_isa, hsa_code_object_version: config.hsa.code_object_version,
			release: config.kernels.release.clone(), fma_chain_length: config.kernels.fma_chain_length, }) }

	fn reopen_config(&self) -> Result<NativeProbeConfig, String> {
		let profile_path = inspect_private_regular_path(&self.profile_path, "active measured profile")?;
		if profile_path != self.profile_path { return Err(format!(
				"active measured profile path {} no longer resolves exactly",
				self.profile_path.display() )); }
		let pci_sysfs_root = inspect_canonical_directory(&self.pci_sysfs_root, "active PCI sysfs root")?;
		let scratch_parent = inspect_private_directory(&self.scratch_parent, "active kernel scratch directory")?;
		Ok(NativeProbeConfig { host_memory_key: self.host_memory_key.clone(), pci_sysfs_root, cuda: CudaProbeConfig {
				library: BackendLibrary {
					candidates: reopen_library(&self.cuda_library, "CUDA Driver")?,
				}, ptx_isa: self.ptx_isa, }, hsa: HsaProbeConfig { library: BackendLibrary {
					candidates: reopen_library(&self.hsa_library, "ROCr/HSA runtime")?,
				}, code_object_version: self.hsa_code_object_version, }, kernels: KernelBuildConfig { toolchain: OfflineToolchain {
					verifier: self.verifier.reopen_tool("LLVM opt")?,
					llvm_codegen: self.llvm_codegen.reopen_tool("LLVM llc")?,
					elf_linker: self .elf_linker .as_ref()
						.map(|tool| tool.reopen_tool("LLVM lld"))
						.transpose()?, ptx_assembler: self .ptx_assembler .as_ref()
						.map(|tool| tool.reopen_tool("NVIDIA ptxas"))
						.transpose()?, }, release: self.release.clone(), scratch_parent, fma_chain_length: self.fma_chain_length, }, }) }

	fn encode(&self) -> Vec<u8> { let mut output = String::new();
		writeln!(output, "{ACTIVE_NATIVE_MAGIC}").expect("writing to String");
		write_receipt_field( &mut output,
			"profile",
			&encode_os(self.profile_path.as_os_str()), ); write_receipt_field( &mut output,
			"profile_schema",
			&self.profile_identity.schema.to_string(), ); write_receipt_field( &mut output,
			"profile_digest",
			&encode_hex(&self.profile_identity.digest.bytes()), ); write_receipt_field( &mut output,
			"host_memory_key",
			&encode_hex(self.host_memory_key.as_str().as_bytes()), ); write_receipt_field( &mut output,
			"pci_sysfs_root",
			&encode_os(self.pci_sysfs_root.as_os_str()), ); write_receipt_field( &mut output,
			"scratch_parent",
			&encode_os(self.scratch_parent.as_os_str()), ); write_receipt_field( &mut output,
			"cuda_library",
			&encode_optional_pin(self.cuda_library.as_ref()), ); write_receipt_field( &mut output,
			"hsa_library",
			&encode_optional_pin(self.hsa_library.as_ref()), );
		write_receipt_field(&mut output, "llvm_opt", &encode_pin(&self.verifier));
		write_receipt_field(&mut output, "llvm_llc", &encode_pin(&self.llvm_codegen));
		write_receipt_field( &mut output,
			"lld",
			&encode_optional_pin(self.elf_linker.as_ref()), ); write_receipt_field( &mut output,
			"ptxas",
			&encode_optional_pin(self.ptx_assembler.as_ref()), );
		write_receipt_field(&mut output, "ptx_isa", &self.ptx_isa.to_string());
		write_receipt_field( &mut output,
			"hsa_code_object_version",
			&self.hsa_code_object_version.to_string(), ); write_receipt_field( &mut output,
			"release",
			&encode_hex(self.release.as_str().as_bytes()), ); write_receipt_field( &mut output,
			"fma_chain_length",
			&self.fma_chain_length.to_string(), ); output.into_bytes() }

	fn decode(bytes: &[u8]) -> Result<Self, String> { let text = std::str::from_utf8(bytes)
			.map_err(|error| format!("active native receipt is not canonical UTF-8 text: {error}"))?;
		let mut lines = text.lines(); if lines.next() != Some(ACTIVE_NATIVE_MAGIC) {
			return Err("active native receipt has an invalid schema marker".to_owned());
		}
		let mut values = Vec::with_capacity(RECEIPT_FIELDS.len()); for expected in RECEIPT_FIELDS { let line = lines .next()
				.ok_or_else(|| format!("active native receipt is missing field {expected}"))?;
			let (field, value) = line
				.split_once('\t')
				.ok_or_else(|| format!("active native receipt field {expected} has no tab separator"))?;
			if field != expected { return Err(format!(
					"active native receipt expected field {expected}, found {field}"
				)); }
			values.push(value); }
		if lines.next().is_some() {
			return Err("active native receipt contains unknown trailing fields".to_owned());
		}
		let receipt = Self {
			profile_path: decode_path(values[0], "profile")?,
			profile_identity: CacheIdentity {
				schema: parse_decimal(values[1], "profile_schema")?,
				digest: Digest::new(decode_digest(values[2], "profile_digest")?),
			},
			host_memory_key: decode_label(values[3], "host_memory_key")?,
			pci_sysfs_root: decode_path(values[4], "pci_sysfs_root")?,
			scratch_parent: decode_path(values[5], "scratch_parent")?,
			cuda_library: decode_optional_pin(values[6], "cuda_library")?,
			hsa_library: decode_optional_pin(values[7], "hsa_library")?,
			verifier: decode_required_pin(values[8], "llvm_opt")?,
			llvm_codegen: decode_required_pin(values[9], "llvm_llc")?,
			elf_linker: decode_optional_pin(values[10], "lld")?,
			ptx_assembler: decode_optional_pin(values[11], "ptxas")?,
			ptx_isa: parse_decimal(values[12], "ptx_isa")?,
			hsa_code_object_version: parse_decimal(values[13], "hsa_code_object_version")?,
			release: decode_label(values[14], "release")?,
			fma_chain_length: parse_decimal(values[15], "fma_chain_length")?,
		}; if receipt.encode() != bytes {
			return Err("active native receipt is not in canonical v1 encoding".to_owned());
		}
		Ok(receipt) } }

impl PinnedNativeFile { fn from_tool(tool: &PinnedTool, name: &str) -> Result<Self, String> {
		let inspected = PinnedTool::inspect(&tool.path).map_err(|error| format!("reinspect {name}: {error}"))?;
		if inspected.path != tool.path || inspected.digest != tool.digest { return Err(format!(
				"{name} changed while the active native receipt was being captured"
			)); }
		Ok(Self { path: inspected.path, digest: inspected.digest.bytes(), }) }

	fn reopen_tool(&self, name: &str) -> Result<PinnedTool, String> { let inspected =
			PinnedTool::inspect(&self.path).map_err(|error| format!("reinspect active {name}: {error}"))?;
		if inspected.path != self.path { return Err(format!(
				"active {name} path {} resolves to {}",
				self.path.display(), inspected.path.display() )); }
		if inspected.digest.bytes() != self.digest { return Err(format!(
				"active {name} digest changed at {}; rerun `recipe probe`",
				self.path.display() )); }
		Ok(inspected) } }

fn profile_uses_backend(profile: &MeasuredProfile, backend: &str) -> bool {
	profile.discovery.devices.iter().any(|device| { device.calculation .as_ref()
			.is_some_and(|calculation| calculation.target.backend.as_str() == backend) }) }

fn required_selected_library(library: &BackendLibrary, name: &str) -> Result<PinnedNativeFile, String> {
	for candidate in &library.candidates { let metadata = match fs::symlink_metadata(candidate) { Ok(metadata) => metadata,
			Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue, Err(error) => { return Err(format!(
					"inspect selected {name} candidate {}: {error}",
					candidate.display() )); } }; if !(metadata.file_type().is_file() || metadata.file_type().is_symlink()) {
			return Err(format!(
				"selected {name} candidate {} is not a file or symlink",
				candidate.display() )); }
		let path = fs::canonicalize(candidate).map_err(|error| { format!(
				"canonicalize selected {name} candidate {}: {error}",
				candidate.display() ) })?; let target = fs::metadata(&path)
			.map_err(|error| format!("inspect selected {name} target {}: {error}", path.display()))?;
		if !target.is_file() { return Err(format!(
				"selected {name} target {} is not a regular file",
				path.display() )); }
		let bytes = fs::read(&path)
			.map_err(|error| format!("read selected {name} target {}: {error}", path.display()))?;
		return Ok(PinnedNativeFile { path, digest: ArtifactDigest::of(&bytes).bytes(), }); }
	Err(format!(
		"profile contains {name} devices but no configured native library remains; rerun `recipe probe`"
	)) }

fn reopen_library(pin: &Option<PinnedNativeFile>, name: &str) -> Result<Vec<PathBuf>, String> {
	let Some(pin) = pin else { return Ok(Vec::new()); }; let current = inspect_pinned_regular_file(&pin.path, name)?;
	if current.digest != pin.digest { return Err(format!(
			"active {name} digest changed at {}; rerun `recipe probe`",
			pin.path.display() )); }
	Ok(vec![current.path]) }

fn inspect_pinned_regular_file(path: &Path, name: &str) -> Result<PinnedNativeFile, String> { if !path.is_absolute() {
		return Err(format!(
			"active {name} path {} is not absolute",
			path.display() )); }
	let metadata =
		fs::symlink_metadata(path).map_err(|error| format!("inspect active {name} {}: {error}", path.display()))?;
	if metadata.file_type().is_symlink() || !metadata.is_file() { return Err(format!(
			"active {name} path {} must remain a regular non-symlink file",
			path.display() )); }
	let canonical = fs::canonicalize(path)
		.map_err(|error| format!("canonicalize active {name} {}: {error}", path.display()))?;
	if canonical != path { return Err(format!(
			"active {name} path {} no longer resolves exactly",
			path.display() )); }
	let bytes = fs::read(path).map_err(|error| format!("read active {name} {}: {error}", path.display()))?;
	Ok(PinnedNativeFile { path: canonical, digest: ArtifactDigest::of(&bytes).bytes(), }) }

fn inspect_private_regular_path(path: &Path, name: &str) -> Result<PathBuf, String> { if !path.is_absolute() {
		return Err(format!("{name} path {} is not absolute", path.display()));
	}
	let metadata =
		fs::symlink_metadata(path).map_err(|error| format!("inspect {name} {}: {error}", path.display()))?;
	if metadata.file_type().is_symlink() || !metadata.is_file() { return Err(format!(
			"{name} path {} must be a regular non-symlink file",
			path.display() )); }
	if metadata.permissions().mode() & PRIVATE_DIRECTORY_MASK != 0 { return Err(format!(
			"{name} path {} grants group or other permissions",
			path.display() )); }
	require_effective_user(metadata.uid(), path, name)?; let canonical =
		fs::canonicalize(path).map_err(|error| format!("canonicalize {name} {}: {error}", path.display()))?;
	if canonical != path {
		return Err(format!("{name} path {} is not canonical", path.display()));
	}
	Ok(canonical) }

fn inspect_canonical_directory(path: &Path, name: &str) -> Result<PathBuf, String> { if !path.is_absolute() {
		return Err(format!("{name} {} is not absolute", path.display()));
	}
	let metadata =
		fs::symlink_metadata(path).map_err(|error| format!("inspect {name} {}: {error}", path.display()))?;
	if metadata.file_type().is_symlink() || !metadata.is_dir() { return Err(format!(
			"{name} {} must be a real directory",
			path.display() )); }
	let canonical =
		fs::canonicalize(path).map_err(|error| format!("canonicalize {name} {}: {error}", path.display()))?;
	if canonical != path {
		return Err(format!("{name} {} is not canonical", path.display()));
	}
	Ok(canonical) }

fn inspect_private_directory(path: &Path, name: &str) -> Result<PathBuf, String> {
	let canonical = inspect_canonical_directory(path, name)?; let metadata = fs::symlink_metadata(&canonical)
		.map_err(|error| format!("inspect {name} {}: {error}", canonical.display()))?;
	if metadata.permissions().mode() & PRIVATE_DIRECTORY_MASK != 0 { return Err(format!(
			"{name} {} grants group or other permissions",
			canonical.display() )); }
	require_effective_user(metadata.uid(), &canonical, name)?; Ok(canonical) }

fn require_effective_user(actual_uid: u32, path: &Path, name: &str) -> Result<(), String> {
	let effective_uid = fs::metadata("/proc/self")
		.map_err(|error| format!("inspect /proc/self ownership: {error}"))?
		.uid(); if actual_uid != effective_uid { return Err(format!(
			"{name} {} is not owned by the effective user",
			path.display() )); }
	Ok(()) }

fn write_active_native_receipt(state_root: &Path, receipt: &ActiveNativeReceipt) -> Result<(), String> {
	inspect_private_directory(state_root, "Recipe state directory")?;
	let target = state_root.join(ACTIVE_NATIVE_RECEIPT); let mut temporary = PendingReceipt::create(state_root)?;
	let bytes = receipt.encode(); temporary .file .as_mut()
		.expect("active receipt temporary is open")
		.write_all(&bytes) .map_err(|error| { format!(
				"write active native receipt temporary {}: {error}",
				temporary.path.display() ) })?; let file = temporary .file .take()
		.expect("active receipt temporary is open");
	file.sync_all().map_err(|error| { format!(
			"sync active native receipt temporary {}: {error}",
			temporary.path.display() ) })?; drop(file); fs::rename(&temporary.path, &target).map_err(|error| { format!(
			"atomically replace active native receipt {}: {error}",
			target.display() ) })?; temporary.committed = true; File::open(state_root)
		.and_then(|directory| directory.sync_all()) .map_err(|error| { format!(
				"sync active native receipt directory {}: {error}",
				state_root.display() ) })?; let metadata =
		fs::symlink_metadata(&target).map_err(|error| format!("inspect active native receipt: {error}"))?;
	if metadata.file_type().is_symlink() || !metadata.is_file()
		|| metadata.permissions().mode() & 0o777 != PRIVATE_FILE_MODE
	{ return Err(format!(
			"active native receipt {} was not installed as a private mode-0600 regular file",
			target.display() )); }
	require_effective_user(metadata.uid(), &target, "active native receipt")
}

fn load_active_native_receipt(state_root: &Path) -> Result<Option<ActiveNativeReceipt>, String> {
	inspect_private_directory(state_root, "Recipe state directory")?;
	let path = state_root.join(ACTIVE_NATIVE_RECEIPT); let before = match fs::symlink_metadata(&path) {
		Ok(metadata) => metadata, Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
		Err(error) => { return Err(format!(
				"inspect active native receipt {}: {error}",
				path.display() )); } }; if before.file_type().is_symlink() || !before.is_file() { return Err(format!(
			"active native receipt {} must be a regular non-symlink file",
			path.display() )); }
	if before.permissions().mode() & 0o777 != PRIVATE_FILE_MODE { return Err(format!(
			"active native receipt {} must have mode 0600, found {:o}",
			path.display(), before.permissions().mode() & 0o777 )); }
	require_effective_user(before.uid(), &path, "active native receipt")?;
	if before.len() > ACTIVE_NATIVE_MAXIMUM_BYTES as u64 { return Err(format!(
			"active native receipt is {} bytes; limit is {ACTIVE_NATIVE_MAXIMUM_BYTES}",
			before.len() )); }
	let file = OpenOptions::new() .read(true) .custom_flags(O_NOFOLLOW | O_CLOEXEC) .open(&path)
		.map_err(|error| format!("open active native receipt {}: {error}", path.display()))?;
	let opened = file.metadata().map_err(|error| { format!(
			"inspect opened active native receipt {}: {error}",
			path.display() ) })?; if opened.dev() != before.dev() || opened.ino() != before.ino() {
		return Err("active native receipt changed while it was opened".to_owned());
	}
	let mut bytes = Vec::with_capacity(
		usize::try_from(opened.len()).map_err(|_| "active native receipt length does not fit usize".to_owned())?,
	); file.take((ACTIVE_NATIVE_MAXIMUM_BYTES as u64) + 1) .read_to_end(&mut bytes)
		.map_err(|error| format!("read active native receipt {}: {error}", path.display()))?;
	if bytes.len() > ACTIVE_NATIVE_MAXIMUM_BYTES { return Err(format!(
			"active native receipt grew beyond the {ACTIVE_NATIVE_MAXIMUM_BYTES}-byte limit while being read"
		)); }
	ActiveNativeReceipt::decode(&bytes).map(Some) }

#[derive(Debug)]
struct PendingReceipt { path: PathBuf, file: Option<File>, committed: bool, }

impl PendingReceipt { fn create(parent: &Path) -> Result<Self, String> { static NEXT: AtomicU64 = AtomicU64::new(1);
		for _ in 0..64 { let nonce = NEXT.fetch_add(1, Ordering::Relaxed); let path = parent.join(format!(
				".{ACTIVE_NATIVE_RECEIPT}.tmp.{}.{nonce}",
				std::process::id() )); match OpenOptions::new() .read(true) .write(true) .create_new(true) .mode(PRIVATE_FILE_MODE)
				.custom_flags(O_NOFOLLOW | O_CLOEXEC) .open(&path)
			{ Ok(file) => { file.set_permissions(fs::Permissions::from_mode(PRIVATE_FILE_MODE)) .map_err(|error| { format!(
								"set active native receipt temporary permissions {}: {error}",
								path.display() ) })?; return Ok(Self { path, file: Some(file), committed: false, }); }
				Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
				Err(error) => { return Err(format!(
						"create active native receipt temporary {}: {error}",
						path.display() )); } } }
		Err("could not allocate a unique active native receipt temporary file".to_owned())
	} }

impl Drop for PendingReceipt { fn drop(&mut self) { if !self.committed { let _ = fs::remove_file(&self.path); } } }

fn write_receipt_field(output: &mut String, name: &str, value: &str) {
	writeln!(output, "{name}\t{value}").expect("writing to String");
}

fn encode_optional_pin(pin: Option<&PinnedNativeFile>) -> String { pin.map_or_else(|| "none".to_owned(), encode_pin) }

fn encode_pin(pin: &PinnedNativeFile) -> String { format!(
		"{}:{}",
		encode_os(pin.path.as_os_str()), encode_hex(&pin.digest) ) }

fn decode_optional_pin(value: &str, field: &str) -> Result<Option<PinnedNativeFile>, String> {
	if value == "none" {
		Ok(None) } else { decode_pin(value, field).map(Some) } }

fn decode_required_pin(value: &str, field: &str) -> Result<PinnedNativeFile, String> {
	if value == "none" {
		Err(format!(
			"active native receipt field {field} cannot be absent"
		)) } else { decode_pin(value, field) } }

fn decode_pin(value: &str, field: &str) -> Result<PinnedNativeFile, String> { let (path, digest) = value
		.split_once(':')
		.ok_or_else(|| format!("active native receipt field {field} has no digest separator"))?;
	Ok(PinnedNativeFile { path: decode_path(path, field)?, digest: decode_digest(digest, field)?, }) }

fn encode_os(value: &OsStr) -> String { encode_hex(value.as_bytes()) }

fn encode_hex(bytes: &[u8]) -> String { let mut output = String::with_capacity(bytes.len() * 2); for byte in bytes {
		write!(output, "{byte:02x}").expect("writing to String");
	}
	output }

fn decode_path(value: &str, field: &str) -> Result<PathBuf, String> { let bytes = decode_hex(value, field)?;
	if bytes.is_empty() { return Err(format!(
			"active native receipt field {field} is an empty path"
		)); }
	Ok(PathBuf::from(OsString::from_vec(bytes))) }

fn decode_label(value: &str, field: &str) -> Result<Label, String> { let bytes = decode_hex(value, field)?;
	let text = String::from_utf8(bytes)
		.map_err(|error| format!("active native receipt field {field} is not UTF-8: {error}"))?;
	Label::new(text).map_err(|error| format!("active native receipt field {field}: {error}"))
}

fn decode_digest(value: &str, field: &str) -> Result<[u8; 32], String> { if value.len() != 64 { return Err(format!(
			"active native receipt field {field} digest must contain 64 lowercase hexadecimal digits"
		)); }
	let bytes = decode_hex(value, field)?; bytes.try_into()
		.map_err(|_| format!("active native receipt field {field} digest does not contain exactly 32 bytes"))
}

fn decode_hex(value: &str, field: &str) -> Result<Vec<u8>, String> { if value.len() % 2 != 0 || !value .bytes()
			.all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
	{ return Err(format!(
			"active native receipt field {field} is not lowercase hexadecimal"
		)); }
	let mut bytes = Vec::with_capacity(value.len() / 2); for pair in value.as_bytes().chunks_exact(2) {
		bytes.push((receipt_hex_nibble(pair[0]) << 4) | receipt_hex_nibble(pair[1])); }
	Ok(bytes) }

const fn receipt_hex_nibble(byte: u8) -> u8 { match byte {
		b'0'..=b'9' => byte - b'0',
		b'a'..=b'f' => byte - b'a' + 10,
		_ => 0, } }

fn parse_decimal<T>(value: &str, field: &str) -> Result<T, String>
where T: std::str::FromStr, T::Err: std::fmt::Display, { value.parse::<T>()
		.map_err(|error| format!("active native receipt field {field} is not a valid decimal integer: {error}"))
}

fn native_config( options: &ProbeOptions, host_memory_key: Label, scratch_parent: PathBuf,
) -> Result<NativeProbeConfig, String> { let verifier = required_tool( options.llvm_opt.as_deref(), &[
			"/usr/bin/opt",
			"/usr/local/bin/opt",
			"/usr/lib/llvm-22/bin/opt",
			"/usr/lib/llvm-21/bin/opt",
			"/usr/lib/llvm-20/bin/opt",
			"/usr/lib/llvm-19/bin/opt",
			"/opt/llvm/bin/opt",
		],
		"LLVM opt",
	)?; let llvm_codegen = required_tool( options.llvm_llc.as_deref(), &[
			"/usr/bin/llc",
			"/usr/local/bin/llc",
			"/usr/lib/llvm-22/bin/llc",
			"/usr/lib/llvm-21/bin/llc",
			"/usr/lib/llvm-20/bin/llc",
			"/usr/lib/llvm-19/bin/llc",
			"/opt/llvm/bin/llc",
		],
		"LLVM llc",
	)?; let elf_linker = optional_tool( options.lld.as_deref(), &[
			"/usr/bin/ld.lld",
			"/usr/local/bin/ld.lld",
			"/usr/lib/llvm-22/bin/ld.lld",
			"/usr/lib/llvm-21/bin/ld.lld",
			"/usr/lib/llvm-20/bin/ld.lld",
			"/opt/llvm/bin/ld.lld",
		],
		"LLVM lld",
	)?; let ptx_assembler = optional_tool( options.ptxas.as_deref(), &[
			"/opt/cuda-11.8/bin/ptxas",
			"/opt/cuda-11.7/bin/ptxas",
			"/opt/cuda-11.6/bin/ptxas",
			"/opt/cuda-11.5/bin/ptxas",
			"/opt/cuda-11.4/bin/ptxas",
			"/usr/local/cuda-11.8/bin/ptxas",
			"/usr/local/cuda-11.4/bin/ptxas",
			"/opt/cuda/bin/ptxas",
			"/usr/local/cuda/bin/ptxas",
			"/usr/bin/ptxas",
		],
		"NVIDIA ptxas",
	)?;

	let cuda_candidates = configured_or_default( &options.cuda_libraries, &[
			"/usr/lib/x86_64-linux-gnu/libcuda.so.1",
			"/usr/lib64/libcuda.so.1",
			"/usr/lib/libcuda.so.1",
			"/usr/local/nvidia/lib64/libcuda.so.1",
		], ); let hsa_candidates = configured_or_default( &options.hsa_libraries, &[
			"/opt/rocm/lib/libhsa-runtime64.so.1",
			"/usr/lib/x86_64-linux-gnu/libhsa-runtime64.so.1",
			"/usr/lib64/libhsa-runtime64.so.1",
			"/usr/lib/libhsa-runtime64.so.1",
		], ); Ok(NativeProbeConfig { host_memory_key,
		pci_sysfs_root: PathBuf::from("/sys/bus/pci/devices"),
		cuda: CudaProbeConfig { library: BackendLibrary { candidates: cuda_candidates, },
			// PTX 7.4 is accepted by the R470 deployment fixture and remains
			// valid input to newer pinned assemblers.
			ptx_isa: 74, }, hsa: HsaProbeConfig { library: BackendLibrary { candidates: hsa_candidates, },
			code_object_version: 6, }, kernels: KernelBuildConfig { toolchain: OfflineToolchain { verifier, llvm_codegen,
				elf_linker, ptx_assembler, },
			release: Label::new("auto-pinned-local-tools-and-benchmark-v3").map_err(|error| error.to_string())?,
			scratch_parent,
			// The structurally inspected HSACO path is live-validated through
			// 64 dependent FMAs on gfx1101. This is long enough to amortize
			// launch overhead while keeping the bounded probe responsive.
			fma_chain_length: 64, }, }) }

fn configured_or_default(configured: &[PathBuf], defaults: &[&str]) -> Vec<PathBuf> { if configured.is_empty() {
		defaults.iter().map(PathBuf::from).collect() } else { configured.to_vec() } }

fn required_tool(explicit: Option<&Path>, candidates: &[&str], name: &str) -> Result<PinnedTool, String> {
	optional_tool(explicit, candidates, name)?
		.ok_or_else(|| format!("{name} was not found in the fixed candidate set; supply its exact absolute path"))
}

fn optional_tool(explicit: Option<&Path>, candidates: &[&str], name: &str) -> Result<Option<PinnedTool>, String> {
	if let Some(path) = explicit { return PinnedTool::inspect(path) .map(Some)
			.map_err(|error| format!("{name}: {error}"));
	}
	for candidate in candidates { let path = Path::new(candidate); match fs::symlink_metadata(path) { Ok(_) => {
				return PinnedTool::inspect(path) .map(Some)
					.map_err(|error| format!("{name}: {error}"));
			}
			Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
			Err(error) => { return Err(format!(
					"inspect {name} candidate {}: {error}",
					path.display() )); } } }
	Ok(None) }

fn private_state_root() -> Result<PathBuf, String> {
	let base = match env::var_os("XDG_CACHE_HOME") {
		Some(value) if Path::new(&value).is_absolute() => PathBuf::from(value), Some(_) | None => {
			let home = env::var_os("HOME")
				.ok_or_else(|| "neither an absolute XDG_CACHE_HOME nor HOME is available".to_owned())?;
			let home = fs::canonicalize(&home).map_err(|error| format!("canonicalize user home: {error}"))?;
			home.join(".cache")
		} }; if !base.exists() { DirBuilder::new() .recursive(true) .mode(0o700) .create(&base)
			.map_err(|error| format!("create cache base {}: {error}", base.display()))?;
	}
	let base =
		fs::canonicalize(&base).map_err(|error| format!("canonicalize cache base {}: {error}", base.display()))?;
	let root = base.join("recipe-next");
	ensure_private_directory(&root)?; Ok(root) }

fn ensure_private_directory(path: &Path) -> Result<(), String> { if !path.is_absolute() { return Err(format!(
			"private directory {} is not absolute",
			path.display() )); }
	if !path.exists() { DirBuilder::new() .mode(0o700) .create(path)
			.map_err(|error| format!("create private directory {}: {error}", path.display()))?;
	}
	let metadata = fs::symlink_metadata(path)
		.map_err(|error| format!("inspect private directory {}: {error}", path.display()))?;
	if metadata.file_type().is_symlink() || !metadata.is_dir() { return Err(format!(
			"private path {} must be a real directory",
			path.display() )); }
	if metadata.permissions().mode() & 0o077 != 0 { return Err(format!(
			"private directory {} grants group or other permissions",
			path.display() )); }
	let effective_uid = fs::metadata("/proc/self")
		.map_err(|error| format!("inspect /proc/self ownership: {error}"))?
		.uid(); if metadata.uid() != effective_uid { return Err(format!(
			"private directory {} is not owned by the effective user",
			path.display() )); }
	let canonical = fs::canonicalize(path)
		.map_err(|error| format!("canonicalize private directory {}: {error}", path.display()))?;
	if canonical != path { return Err(format!(
			"private directory {} is not canonical or contains a symlink",
			path.display() )); }
	Ok(()) }

fn require_bare_metal() -> Result<(), String> {
	for marker in ["/.dockerenv", "/run/.containerenv"] {
		if Path::new(marker).exists() { return Err(format!(
				"`recipe probe` requires bare metal; container marker {marker} exists"
			)); } }
	if let Ok(cgroup) = fs::read_to_string("/proc/1/cgroup") {
		let lower = cgroup.to_ascii_lowercase();
		for marker in ["docker", "containerd", "kubepods", "libpod", "lxc"] {
			if lower.contains(marker) { return Err(format!(
					"`recipe probe` requires bare metal; PID 1 cgroup reports {marker}"
				)); } } }
	if let Ok(status) = fs::read_to_string("/proc/self/status")
		&& let Some(line) = status.lines().find(|line| line.starts_with("NSpid:"))
		&& line.split_ascii_whitespace().skip(1).count() > 1
	{
		return Err("`recipe probe` requires bare metal; the process is inside a PID namespace".to_owned());
	}
	Ok(()) }

fn hex(digest: Digest) -> String { let mut output = String::with_capacity(64); for byte in digest.bytes() {
		write!(output, "{byte:02x}").expect("writing to String");
	}
	output }
