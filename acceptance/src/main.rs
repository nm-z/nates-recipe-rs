use std::{
	collections::BTreeMap,
	env,
	error::Error,
	fs,
	io::{self, BufRead, BufReader, Read},
	path::{Path, PathBuf},
	process::{Command, Stdio},
	sync::mpsc,
	thread::{self, JoinHandle},
	time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use recipe::{
	engine::executor::{LogicalEvent, PhysicalCall, RunJournal},
	*,
};
use sha2::{Digest as _, Sha256};

const REQUIRED_SAMPLES: usize = 7;
const REQUIRED_INPUTS: usize = 5;
const LLAMA_CPP_REVISION: &str = include_str!("../llama.cpp-revision");
const LLAMA_POSITIONS: usize = 128;
const LLAMA_VOCABULARY: usize = 128;
const MAXIMUM_LLAMA_NMSE: f64 = 1.0e-3;

type AcceptanceResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug)]
struct Inputs {
	dataset_root: PathBuf,
	digest_manifest: PathBuf,
	recipe_cli: PathBuf,
	llama_oracle: PathBuf,
	samples: usize,
}

#[derive(Debug)]
struct TimedLogits {
	elapsed: Duration,
	values: Vec<f32>,
	native: NativeRunRecord,
}

#[derive(Clone, Copy, Debug)]
struct NativeRunRecord {
	image_loads: usize,
	entry_lookups: usize,
	queues: usize,
	completion_objects: usize,
	persistent_allocations: usize,
	calculation_submissions: usize,
	transfer_submissions: usize,
	metric_submissions: usize,
}

fn main() -> AcceptanceResult<()> {
	let mut record = AcceptanceRecord::new()?;
	record.capture_provenance();
	let outcome = run_acceptance(&mut record);
	match &outcome {
		Ok(()) => record.field("result", "pass"),
		Err(error) => {
			record.field("result", "fail");
			record.field("failure_reason", error.to_string());
		}
	}
	record.persist()?;
	outcome
}

fn run_acceptance(record: &mut AcceptanceRecord) -> AcceptanceResult<()> {
	let inputs = parse_arguments()?;
	record.field("backend", "cuda");
	record.field("measured_samples", inputs.samples);
	record.field("warmup_policy", "one untimed warm-up per implementation");
	record.field("digest_manifest", inputs.digest_manifest.display());
	record.field("recipe_cli", inputs.recipe_cli.display());
	record.field("llama_oracle", inputs.llama_oracle.display());
	verify_inputs(&inputs)?;
	verify_oracle_revision(&inputs.llama_oracle)?;
	record.field("llama_cpp_revision", LLAMA_CPP_REVISION.trim());
	record.capture_input_manifest(&inputs)?;
	record.capture_native_profile()?;
	let scratch = tempfile::Builder::new()
		.prefix("recipe-acceptance-cuda-")
		.tempdir()?;
	run_training_gate(&inputs.dataset_root, scratch.path(), record)?;
	run_recurrent_gate(&inputs.dataset_root, scratch.path(), record)?;
	run_artifact_gate(&inputs, scratch.path(), record)?;
	run_llama_gate(&inputs, scratch.path(), record)?;
	println!("acceptance\tbackend\tcuda\tresult\tpass");
	Ok(())
}

#[derive(Debug)]
struct AcceptanceRecord {
	path: PathBuf,
	lines: Vec<String>,
}

impl AcceptanceRecord {
	fn new() -> AcceptanceResult<Self> {
		let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
			.parent()
			.ok_or_else(|| invalid("acceptance package has no workspace parent"))?;
		let target = env::var_os("CARGO_TARGET_DIR").map_or_else(|| workspace.join("target"), PathBuf::from);
		let directory = if target.is_absolute() {
			target.join("acceptance")
		} else {
			workspace.join(target).join("acceptance")
		};
		fs::create_dir_all(&directory)?;
		let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
		let path = directory.join(format!("cuda-{timestamp}-{}.tsv", std::process::id()));
		let mut record = Self {
			path,
			lines: Vec::new(),
		};
		record.field("schema", "recipe-acceptance-v1");
		record.field("unix_timestamp", timestamp);
		Ok(record)
	}

	fn field(&mut self, name: &str, value: impl ToString) {
		let value = value
			.to_string()
			.replace('\\', "\\\\")
			.replace('\t', "\\t")
			.replace('\n', "\\n")
			.replace('\r', "\\r");
		self.lines.push(format!("{name}\t{value}"));
	}

	fn capture_provenance(&mut self) {
		self.field("recipe_version", env!("CARGO_PKG_VERSION"));
		self.field(
			"recipe_profile",
			if cfg!(debug_assertions) {
				"debug"
			} else {
				"release"
			},
		);
		self.command_field("git_commit", "git", &["rev-parse", "HEAD"]);
		let dirty = command_stdout("git", &["status", "--porcelain"]).map_or_else(
			|error| format!("unavailable: {error}"),
			|output| (!output.is_empty()).to_string(),
		);
		self.field("git_dirty", dirty);
		self.command_field("rustc", "rustc", &["-Vv"]);
		self.command_field("nvidia_gpu", "nvidia-smi", &[
			"--query-gpu=name,uuid,pci.bus_id,driver_version,memory.total",
			"--format=csv,noheader,nounits",
		]);
		for (field, program, arguments) in [
			("ptxas", "ptxas", &["--version"][..]),
			("llvm_opt", "opt", &["--version"][..]),
			("llvm_llc", "llc", &["--version"][..]),
			("lld", "ld.lld", &["--version"][..]),
			("mold", "mold", &["--version"][..]),
		] {
			self.command_field(field, program, arguments);
		}
	}

	fn capture_input_manifest(&mut self, inputs: &Inputs) -> AcceptanceResult<()> {
		for line in fs::read_to_string(&inputs.digest_manifest)?.lines() {
			let line = line.trim();
			if let Some((digest, path)) = line.split_once("  ") {
				self.field(&format!("input_sha256_{}", path.replace('/', "_")), digest);
			}
		}
		Ok(())
	}

	fn capture_native_profile(&mut self) -> AcceptanceResult<()> {
		with_current_native_preparation(|profile, _config, scope| {
			self.field("measured_profile_schema", profile.cache_identity.schema);
			self.field(
				"measured_profile_digest",
				hex_digest(profile.cache_identity.digest),
			);
			self.field(
				"measured_topology_digest",
				hex_digest(profile.topology.identity.digest()),
			);
			self.field(
				"measured_discovery_digest",
				hex_digest(profile.discovery.identity.digest()),
			);
			for (index, target) in scope.targets().devices().iter().enumerate() {
				self.field(&format!("measured_device_{index}"), target.origin());
				self.field(
					&format!("measured_target_{index}"),
					format!("{:?}", target.target()),
				);
				self.field(
					&format!("measured_toolchain_{index}"),
					format!("{:?}", target.toolchain()),
				);
			}
			Ok(())
		})?;
		Ok(())
	}

	fn command_field(&mut self, field: &str, program: &str, arguments: &[&str]) {
		let value = command_stdout(program, arguments).unwrap_or_else(|error| format!("unavailable: {error}"));
		self.field(field, value);
	}

	fn persist(&self) -> AcceptanceResult<()> {
		let mut image = self.lines.join("\n");
		image.push('\n');
		fs::write(&self.path, image)?;
		eprintln!("acceptance_record={}", self.path.display());
		Ok(())
	}
}

fn command_stdout(program: &str, arguments: &[&str]) -> io::Result<String> {
	let output = Command::new(program).args(arguments).output()?;
	if !output.status.success() {
		return Err(io::Error::other(format!(
			"{program} exited with {}",
			output.status
		)));
	}
	let mut text = String::from_utf8_lossy(&output.stdout).trim().to_owned();
	if text.is_empty() {
		text = String::from_utf8_lossy(&output.stderr).trim().to_owned();
	}
	Ok(text)
}

fn hex_digest(digest: recipe::engine::core::Digest) -> String {
	let mut output = String::with_capacity(64);
	for byte in digest.bytes() {
		use std::fmt::Write as _;
		write!(output, "{byte:02x}").expect("writing to String");
	}
	output
}

fn parse_arguments() -> AcceptanceResult<Inputs> {
	let mut arguments = env::args_os().skip(1);
	let backend = arguments
		.next()
		.ok_or_else(|| invalid(usage("missing backend")))?;
	if backend != "cuda" {
		return Err(invalid(usage(
			"this machine's acceptance run must name the real `cuda` backend",
		))
		.into());
	}
	let mut values = BTreeMap::<String, PathBuf>::new();
	let mut samples = None;
	while let Some(flag) = arguments.next() {
		let flag = flag
			.into_string()
			.map_err(|_| invalid("acceptance option is not valid UTF-8"))?;
		let value = arguments
			.next()
			.ok_or_else(|| invalid(format!("{flag} requires a value")))?;
		if flag == "--samples" {
			let text = value
				.into_string()
				.map_err(|_| invalid("--samples is not valid UTF-8"))?;
			samples = Some(text.parse::<usize>()?);
		} else if matches!(
			flag.as_str(),
			"--dataset-root" | "--digest-manifest" | "--recipe-cli" | "--llama-oracle"
		) {
			if values.insert(flag.clone(), PathBuf::from(value)).is_some() {
				return Err(invalid(format!("{flag} was declared more than once")).into());
			}
		} else {
			return Err(invalid(usage(&format!("unknown option {flag}"))).into());
		}
	}
	let samples = samples.ok_or_else(|| invalid(usage("--samples is required")))?;
	if samples != REQUIRED_SAMPLES {
		return Err(invalid(format!(
			"acceptance requires exactly {REQUIRED_SAMPLES} measured samples, received {samples}"
		))
		.into());
	}
	Ok(Inputs {
		dataset_root: take_required(&mut values, "--dataset-root")?,
		digest_manifest: take_required(&mut values, "--digest-manifest")?,
		recipe_cli: take_required(&mut values, "--recipe-cli")?,
		llama_oracle: take_required(&mut values, "--llama-oracle")?,
		samples,
	})
}

fn take_required(values: &mut BTreeMap<String, PathBuf>, name: &str) -> AcceptanceResult<PathBuf> {
	values.remove(name)
		.ok_or_else(|| invalid(usage(&format!("{name} is required"))).into())
}

fn usage(detail: &str) -> String {
	format!(
		"{detail}\nusage: recipe-acceptance cuda --dataset-root PATH --digest-manifest PATH \
	--recipe-cli PATH --llama-oracle PATH --samples {REQUIRED_SAMPLES}"
	)
}

fn invalid(detail: impl Into<String>) -> io::Error { io::Error::new(io::ErrorKind::InvalidInput, detail.into()) }

fn verify_inputs(inputs: &Inputs) -> AcceptanceResult<()> {
	let manifest = fs::read_to_string(&inputs.digest_manifest)?;
	let mut observed = 0_usize;
	for (line_index, line) in manifest.lines().enumerate() {
		let line = line.trim();
		if line.is_empty() || line.starts_with('#') {
			continue;
		}
		let (expected, relative) = line.split_once("  ").ok_or_else(|| {
			invalid(format!(
				"digest manifest line {} is not `SHA256  relative/path`",
				line_index + 1
			))
		})?;
		if expected.len() != 64 || !expected.bytes().all(|byte| byte.is_ascii_hexdigit()) {
			return Err(invalid(format!(
				"digest manifest line {} has an invalid SHA-256",
				line_index + 1
			))
			.into());
		}
		let path = inputs.dataset_root.join(relative);
		let bytes = fs::read(&path).map_err(|error| {
			io::Error::new(
				error.kind(),
				format!(
					"read required real acceptance input {}: {error}",
					path.display()
				),
			)
		})?;
		if bytes.starts_with(b"version https://git-lfs.github.com/spec/v1") {
			return Err(invalid(format!(
				"{} is a Git LFS pointer, not the required real input",
				path.display()
			))
			.into());
		}
		let actual = format!("{:x}", Sha256::digest(&bytes));
		if actual != expected {
			return Err(invalid(format!(
				"input digest mismatch for {}: expected {expected}, observed {actual}",
				path.display()
			))
			.into());
		}
		observed += 1;
	}
	if observed != REQUIRED_INPUTS {
		return Err(invalid(format!(
			"digest manifest must name exactly {REQUIRED_INPUTS} required inputs, found {observed}"
		))
		.into());
	}
	if !inputs.llama_oracle.is_file() {
		return Err(invalid(format!(
			"pinned llama.cpp oracle does not exist at {}",
			inputs.llama_oracle.display()
		))
		.into());
	}
	if !inputs.recipe_cli.is_file() {
		return Err(invalid(format!(
			"Recipe CLI does not exist at {}",
			inputs.recipe_cli.display()
		))
		.into());
	}
	Ok(())
}

fn verify_oracle_revision(oracle: &Path) -> AcceptanceResult<()> {
	let output = Command::new(oracle).arg("--revision").output()?;
	if !output.status.success() {
		return Err(invalid(format!(
			"llama.cpp oracle revision query failed with {}: {}",
			output.status,
			String::from_utf8_lossy(&output.stderr)
		))
		.into());
	}
	let expected = LLAMA_CPP_REVISION.trim();
	let actual = String::from_utf8(output.stdout)?.trim().to_owned();
	if actual != expected {
		return Err(invalid(format!(
			"llama.cpp oracle revision mismatch: expected {expected}, observed {actual}"
		))
		.into());
	}
	Ok(())
}

fn run_training_gate(dataset_root: &Path, scratch: &Path, record: &mut AcceptanceRecord) -> AcceptanceResult<()> {
	let dataset = dataset_root.join("uci-airfoil/airfoil_self_noise.dat");
	let model = scratch.join("airfoil.ogdl");
	recipe.data(path_text(&dataset)?)
		.target("col6")
		.norm(min_max)
		.split(0.8);
	recipe.model()
		.perc(24)
		.silu()
		.layer(12)
		.huber()
		.layer(1)
		.loss(huber);
	let report = recipe
		.train()
		.optimizer(adamw)
		.epochs(1)
		.lr(0.0001)
		.cos()
		.log(Loss)
		.save(path_text(&model)?)
		.run()?;
	if report.kind() != TrainingModelKind::Dense {
		return Err(invalid("airfoil acceptance did not execute dense native training").into());
	}
	let journal = report
		.journal()
		.ok_or_else(|| invalid("dense training report omitted its native lifecycle journal"))?;
	verify_completed_lifecycle(journal, 1)?;
	let kernels = report
		.native_kernels()
		.ok_or_else(|| invalid("dense training report omitted its native image evidence"))?;
	verify_one_cuda_image(kernels)?;
	verify_cuda_evidence(
		report.native_evidence()
			.ok_or_else(|| invalid("dense training report omitted driver-derived native evidence"))?,
	)?;
	let evidence = report
		.native_evidence()
		.expect("dense native evidence was checked above");
	let observed_device = &evidence.devices()[0];
	let training_evidence = report
		.training_evidence()
		.ok_or_else(|| invalid("dense training report omitted full-partition execution evidence"))?;
	let bounds = training_evidence.bounds();
	if bounds.train_rows <= 1
		|| training_evidence.logical_updates_per_epoch() != 1
		|| training_evidence.optimizer_parameter_tasks() == 0
		|| training_evidence.optimizer_parameter_submissions() != training_evidence.optimizer_parameter_tasks()
		|| training_evidence.loop_iterations_started() != 1
		|| training_evidence.loop_iterations_completed() != 1
	{
		return Err(invalid(format!(
			"training did not execute one real full-partition update: {training_evidence:?}"
		))
		.into());
	}
	if training_evidence.non_gpu_calculation_tasks() != 0 || training_evidence.non_gpu_calculation_submissions() != 0
	{
		return Err(invalid(format!(
			"training admitted host calculation work: {training_evidence:?}"
		))
		.into());
	}
	if training_evidence.logical_events_compacted() != 0 {
		return Err(invalid(format!(
			"one-epoch logical training evidence was compacted: {training_evidence:?}"
		))
		.into());
	}
	record.field(
		"training_pending_poll_calls_compacted",
		training_evidence.physical_calls_compacted(),
	);
	record.field("training_rows", bounds.train_rows);
	record.field("training_epochs", 1);
	record.field(
		"training_logical_updates_per_epoch",
		training_evidence.logical_updates_per_epoch(),
	);
	record.field(
		"training_optimizer_parameter_tasks",
		training_evidence.optimizer_parameter_tasks(),
	);
	record.field(
		"training_optimizer_parameter_submissions",
		training_evidence.optimizer_parameter_submissions(),
	);
	record.field(
		"training_non_gpu_calculation_tasks",
		training_evidence.non_gpu_calculation_tasks(),
	);
	record.field(
		"training_non_gpu_calculation_submissions",
		training_evidence.non_gpu_calculation_submissions(),
	);
	record.field("training_native_image_loads", observed_device.image_loads);
	record.field(
		"training_native_entry_lookups",
		observed_device.entry_lookups,
	);
	record.field("training_native_queues", observed_device.queues);
	record.field(
		"training_native_completion_objects",
		observed_device.completion_objects,
	);
	record.field(
		"training_native_persistent_allocations",
		observed_device.persistent_allocations,
	);
	record.field(
		"training_loop_realization_calls",
		evidence.loop_realization_calls(),
	);
	record.field(
		"training_live_resources_after_teardown",
		evidence.live_resources_after_teardown(),
	);
	if report.metrics().is_empty()
		|| report.metrics().iter().any(|metric| {
			metric.sample
				.as_ref()
				.is_none_or(|sample| sample.epoch.get() != 1)
		}) {
		return Err(invalid("real training did not retain its full-partition epoch metric").into());
	}
	let files = fs::read_dir(scratch)?.collect::<Result<Vec<_>, _>>()?;
	if files.len() != 1 || files[0].path() != model {
		return Err(invalid("model-only training did not leave exactly the requested .ogdl artifact").into());
	}
	println!(
		"acceptance\tworkload\tuci-airfoil-training\trows\t{}\titerations\t1\tnative_images\t1\tresult\tpass",
		bounds.train_rows
	);
	Ok(())
}

fn run_recurrent_gate(dataset_root: &Path, scratch: &Path, record: &mut AcceptanceRecord) -> AcceptanceResult<()> {
	let root = scratch.join("recurrent-lifecycle");
	fs::create_dir(&root)?;
	let dataset = dataset_root.join("cookbook/binary.csv");
	let model = root.join("gru.ogdl");
	let model_text = path_text(&model)?;
	recipe.data(path_text(&dataset)?)
		.target("target")
		.norm(z_score)
		.split(0.75);
	recipe.model().gru(8).layer(1).loss(bce);
	let training = recipe
		.train()
		.optimizer(adamw)
		.epochs(1)
		.lr(0.001)
		.cos()
		.log(Loss)
		.save(&model_text)
		.run()?;
	if training.kind() != TrainingModelKind::Dense {
		return Err(invalid("GRU acceptance did not execute dense native training").into());
	}
	verify_completed_lifecycle(
		training
			.journal()
			.ok_or_else(|| invalid("GRU training omitted its native lifecycle journal"))?,
		1,
	)?;
	verify_one_cuda_image(
		training
			.native_kernels()
			.ok_or_else(|| invalid("GRU training omitted its bundled native image"))?,
	)?;
	let training_native = training
		.native_evidence()
		.ok_or_else(|| invalid("GRU training omitted driver-derived native evidence"))?;
	verify_cuda_evidence(training_native)?;
	let training_semantics = training
		.training_evidence()
		.ok_or_else(|| invalid("GRU training omitted full-partition execution evidence"))?;
	if training_semantics.bounds().train_rows != 9
		|| training_semantics.logical_updates_per_epoch() != 1
		|| training_semantics.optimizer_parameter_tasks() == 0
		|| training_semantics.optimizer_parameter_submissions() != training_semantics.optimizer_parameter_tasks()
		|| training_semantics.non_gpu_calculation_tasks() != 0
		|| training_semantics.non_gpu_calculation_submissions() != 0
	{
		return Err(invalid(format!(
			"GRU training did not execute one GPU-only full-partition update: {training_semantics:?}"
		))
		.into());
	}

	recipe.data(path_text(&dataset)?).exclude("target");
	recipe.model().load(&model_text);
	let inference = recipe.infer().evaluate()?;
	if inference.kind() != InferenceModelKind::Dense || inference.values().len() != 12 {
		return Err(invalid("saved GRU inference did not return one prediction for every real row").into());
	}
	verify_completed_lifecycle(inference.journal(), 1)?;
	verify_one_cuda_image(
		inference
			.native_kernels()
			.ok_or_else(|| invalid("saved GRU inference omitted its bundled native image"))?,
	)?;
	verify_cuda_evidence(inference.native_evidence())?;
	let files = fs::read_dir(&root)?.collect::<Result<Vec<_>, _>>()?;
	if files.len() != 1 || files[0].path() != model {
		return Err(invalid("GRU train-save-infer left files other than the declared model").into());
	}
	record.field(
		"recurrent_training_rows",
		training_semantics.bounds().train_rows,
	);
	record.field(
		"recurrent_training_native_image_loads",
		training_native.devices()[0].image_loads,
	);
	record.field(
		"recurrent_inference_native_image_loads",
		inference.native_evidence().devices()[0].image_loads,
	);
	record.field("recurrent_inference_rows", inference.values().len());
	println!("acceptance\tworkload\tgru-train-save-infer\trows\t9\tnative_images_per_run\t1\tresult\tpass");
	Ok(())
}

fn run_artifact_gate(inputs: &Inputs, scratch: &Path, record: &mut AcceptanceRecord) -> AcceptanceResult<()> {
	let root = scratch.join("artifact-contract");
	let sources = root.join("sources");
	let outputs = root.join("outputs");
	fs::create_dir_all(&sources)?;
	fs::create_dir_all(&outputs)?;
	let dataset = fs::canonicalize(
		inputs.dataset_root
			.join("uci-airfoil/airfoil_self_noise.dat"),
	)?;

	let none = outputs.join("none");
	fs::create_dir(&none)?;
	run_training_source(
		&inputs.recipe_cli,
		&sources,
		&dataset,
		"no-save",
		None,
		None,
	)?;
	verify_artifact_set(&none, &[])?;

	let model_only = outputs.join("model-only");
	fs::create_dir(&model_only)?;
	let model = model_only.join("model.ogdl");
	run_training_source(
		&inputs.recipe_cli,
		&sources,
		&dataset,
		"model-only",
		None,
		Some(format!(".save({})", rust_path_literal(&model)?)),
	)?;
	verify_artifact_set(&model_only, &["model.ogdl"])?;

	let kernel_only = outputs.join("kernel-only");
	fs::create_dir(&kernel_only)?;
	let kernel = kernel_only.join("kernel.cubin");
	run_training_source(
		&inputs.recipe_cli,
		&sources,
		&dataset,
		"kernel-only",
		None,
		Some(format!(".save({})", rust_path_literal(&kernel)?)),
	)?;
	verify_artifact_set(&kernel_only, &["kernel.cubin"])?;

	let pair = outputs.join("pair");
	fs::create_dir(&pair)?;
	let pair_model = pair.join("model.ogdl");
	let pair_kernel = pair.join("kernel.cubin");
	run_training_source(
		&inputs.recipe_cli,
		&sources,
		&dataset,
		"model-kernel-pair",
		None,
		Some(format!(
			".save({}, {})",
			rust_path_literal(&pair_model)?,
			rust_path_literal(&pair_kernel)?
		)),
	)?;
	verify_artifact_set(&pair, &["kernel.cubin", "model.ogdl"])?;

	let missing = outputs.join("missing-resume");
	fs::create_dir(&missing)?;
	let absent_model = missing.join("absent.ogdl");
	let fresh_model = missing.join("fresh.ogdl");
	run_training_source(
		&inputs.recipe_cli,
		&sources,
		&dataset,
		"missing-resume",
		Some(format!(".resume({})", rust_path_literal(&absent_model)?)),
		Some(format!(".save({})", rust_path_literal(&fresh_model)?)),
	)?;
	verify_artifact_set(&missing, &["fresh.ogdl"])?;

	let resumed = outputs.join("existing-resume");
	fs::create_dir(&resumed)?;
	let resumed_model = resumed.join("resumed.ogdl");
	run_training_source(
		&inputs.recipe_cli,
		&sources,
		&dataset,
		"existing-resume",
		Some(format!(".resume({})", rust_path_literal(&model)?)),
		Some(format!(".save({})", rust_path_literal(&resumed_model)?)),
	)?;
	verify_artifact_set(&resumed, &["resumed.ogdl"])?;

	let resumed_pair = outputs.join("existing-pair-resume");
	fs::create_dir(&resumed_pair)?;
	let resumed_pair_model = resumed_pair.join("resumed.ogdl");
	let resumed_pair_kernel = resumed_pair.join("resumed.cubin");
	run_training_source(
		&inputs.recipe_cli,
		&sources,
		&dataset,
		"existing-pair-resume",
		Some(format!(
			".resume({}, {})",
			rust_path_literal(&pair_model)?,
			rust_path_literal(&pair_kernel)?
		)),
		Some(format!(
			".save({}, {})",
			rust_path_literal(&resumed_pair_model)?,
			rust_path_literal(&resumed_pair_kernel)?
		)),
	)?;
	verify_artifact_set(&resumed_pair, &["resumed.cubin", "resumed.ogdl"])?;

	let absent_kernel = outputs.join("absent-kernel-resume");
	fs::create_dir(&absent_kernel)?;
	let missing_kernel = absent_kernel.join("missing.cubin");
	let recompiled_model = absent_kernel.join("recompiled.ogdl");
	let recompiled_kernel = absent_kernel.join("recompiled.cubin");
	run_training_source(
		&inputs.recipe_cli,
		&sources,
		&dataset,
		"absent-kernel-resume",
		Some(format!(
			".resume({}, {})",
			rust_path_literal(&pair_model)?,
			rust_path_literal(&missing_kernel)?
		)),
		Some(format!(
			".save({}, {})",
			rust_path_literal(&recompiled_model)?,
			rust_path_literal(&recompiled_kernel)?
		)),
	)?;
	verify_artifact_set(&absent_kernel, &["recompiled.cubin", "recompiled.ogdl"])?;

	let safe_stop = outputs.join("safe-stop");
	fs::create_dir(&safe_stop)?;
	let safe_model = safe_stop.join("safe.ogdl");
	run_safe_stop_source(&inputs.recipe_cli, &sources, &dataset, &safe_model)?;
	verify_artifact_set(&safe_stop, &["safe.ogdl"])?;

	let safe_resume = outputs.join("safe-stop-resume");
	fs::create_dir(&safe_resume)?;
	let safe_resumed_model = safe_resume.join("resumed.ogdl");
	run_training_source(
		&inputs.recipe_cli,
		&sources,
		&dataset,
		"safe-stop-resume",
		Some(format!(".resume({})", rust_path_literal(&safe_model)?)),
		Some(format!(
			".save({})",
			rust_path_literal(&safe_resumed_model)?
		)),
	)?;
	verify_artifact_set(&safe_resume, &["resumed.ogdl"])?;

	record.field("artifact_cases_completed", 9);
	record.field("artifact_unexpected_files", 0);
	record.field("artifact_safe_stop_epoch_boundary", true);
	record.field("artifact_safe_stop_model_loadable", true);
	println!("acceptance\tworkload\tartifact-save-resume\tcases\t9\tresult\tpass");
	Ok(())
}

fn run_training_source(
	recipe_cli: &Path,
	source_root: &Path,
	dataset: &Path,
	name: &str,
	resume: Option<String>,
	save: Option<String>,
) -> AcceptanceResult<()> {
	let source = source_root.join(format!("{name}.rs"));
	let resume = resume.unwrap_or_default();
	let save = save.unwrap_or_default();
	let program = format!(
		"use recipe::*;\n\
		 fn main() -> TrainingResult<()> {{\n\
		 \trecipe.data({}).target(\"col6\").norm(min_max).split(0.8);\n\
		 \trecipe.model().layer(1).loss(huber);\n\
		 \trecipe.train().optimizer(adamw).epochs(1).lr(0.0001){}{}.run()?;\n\
		 \tOk(())\n\
		 }}\n",
		rust_path_literal(dataset)?,
		resume,
		save,
	);
	fs::write(&source, program)?;
	let output = Command::new(recipe_cli).arg("run").arg(&source).output()?;
	if !output.status.success() {
		return Err(invalid(format!(
			"artifact case {name} failed through `recipe run` with {}: stdout={} stderr={}",
			output.status,
			String::from_utf8_lossy(&output.stdout),
			String::from_utf8_lossy(&output.stderr)
		))
		.into());
	}
	Ok(())
}

fn run_safe_stop_source(recipe_cli: &Path, source_root: &Path, dataset: &Path, model: &Path) -> AcceptanceResult<()> {
	let source = source_root.join("safe-stop.rs");
	let program = format!(
		"use recipe::*;\n\
		 fn main() -> TrainingResult<()> {{\n\
		 \trecipe.data({}).target(\"col6\").norm(min_max).split(0.8);\n\
		 \trecipe.model().layer(1).loss(huber);\n\
		 \tlet report = recipe.train().optimizer(adamw).lr(0.0001).log(Epoch).save({}).run()?;\n\
		 \tassert!(report.gracefully_stopped(), \"SIGINT was not accepted at a complete epoch boundary\");\n\
		 \tlet evidence = report.native_evidence().expect(\"dense safe-stop run omitted native evidence\");\n\
		 \tassert_eq!(evidence.devices().len(), 1, \"safe-stop run did not use exactly one CUDA device\");\n\
		 \tassert_eq!(evidence.devices()[0].image_loads, 1, \"safe-stop run loaded more than one native image\");\n\
		 \tassert_eq!(evidence.loop_realization_calls(), 0, \"safe-stop run realized native resources in its loop\");\n\
		 \tassert_eq!(evidence.live_resources_after_teardown(), 0, \"safe-stop run retained native resources after teardown\");\n\
		 \tprintln!(\"recipe-acceptance-safe-stop-ok\");\n\
		 \tOk(())\n\
		 }}\n",
		rust_path_literal(dataset)?,
		rust_path_literal(model)?,
	);
	fs::write(&source, program)?;

	let mut child = Command::new(recipe_cli)
		.arg("run")
		.arg(&source)
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn()?;
	let stdout = child
		.stdout
		.take()
		.ok_or_else(|| invalid("safe-stop source omitted its stdout pipe"))?;
	let stderr = child
		.stderr
		.take()
		.ok_or_else(|| invalid("safe-stop source omitted its stderr pipe"))?;
	let (line_sender, line_receiver) = mpsc::channel();
	let stdout_reader = thread::spawn(move || -> io::Result<String> {
		let mut capture = String::new();
		for line in BufReader::new(stdout).lines() {
			let line = line?;
			capture.push_str(&line);
			capture.push('\n');
			let _ = line_sender.send(line);
		}
		Ok(capture)
	});
	let stderr_reader = thread::spawn(move || -> io::Result<String> {
		let mut capture = String::new();
		BufReader::new(stderr).read_to_string(&mut capture)?;
		Ok(capture)
	});

	let epoch_deadline = Instant::now() + Duration::from_secs(60);
	let epoch_observed = loop {
		let remaining = epoch_deadline.saturating_duration_since(Instant::now());
		if remaining.is_zero() {
			break false;
		}
		match line_receiver.recv_timeout(remaining) {
			Ok(line) if line.contains("epoch") => break true,
			Ok(_) => {}
			Err(_) => break false,
		}
	};
	if !epoch_observed {
		let _ = child.kill();
		let _ = child.wait();
		let stdout = join_capture(stdout_reader, "stdout")?;
		let stderr = join_capture(stderr_reader, "stderr")?;
		return Err(invalid(format!(
			"safe-stop run did not expose a complete epoch within 60 seconds: stdout={stdout} stderr={stderr}"
		))
		.into());
	}

	let interrupt = Command::new("kill")
		.arg("-INT")
		.arg(child.id().to_string())
		.output()?;
	if !interrupt.status.success() {
		let _ = child.kill();
		let _ = child.wait();
		return Err(invalid(format!(
			"send SIGINT to recipe run process {}: {}",
			child.id(),
			String::from_utf8_lossy(&interrupt.stderr)
		))
		.into());
	}

	let stop_deadline = Instant::now() + Duration::from_secs(60);
	let status = loop {
		if let Some(status) = child.try_wait()? {
			break status;
		}
		if Instant::now() >= stop_deadline {
			let _ = child.kill();
			let _ = child.wait();
			let stdout = join_capture(stdout_reader, "stdout")?;
			let stderr = join_capture(stderr_reader, "stderr")?;
			return Err(invalid(format!(
				"safe-stop run did not finish teardown within 60 seconds: stdout={stdout} stderr={stderr}"
			))
			.into());
		}
		thread::sleep(Duration::from_millis(10));
	};
	let stdout = join_capture(stdout_reader, "stdout")?;
	let stderr = join_capture(stderr_reader, "stderr")?;
	if !status.success() || !stdout.contains("recipe-acceptance-safe-stop-ok") {
		return Err(invalid(format!(
			"safe-stop run failed through `recipe run` with {status}: stdout={stdout} stderr={stderr}"
		))
		.into());
	}
	Ok(())
}

fn join_capture(handle: JoinHandle<io::Result<String>>, stream: &str) -> AcceptanceResult<String> {
	handle.join()
		.map_err(|_| invalid(format!("safe-stop {stream} capture thread panicked")))?
		.map_err(Into::into)
}

fn verify_artifact_set(directory: &Path, expected: &[&str]) -> AcceptanceResult<()> {
	let mut observed = fs::read_dir(directory)?
		.map(|entry| {
			entry?.file_name()
				.into_string()
				.map_err(|_| invalid("artifact file name is not valid UTF-8"))
		})
		.collect::<Result<Vec<_>, _>>()?;
	observed.sort();
	if observed != expected {
		return Err(invalid(format!(
			"artifact directory {} contains {observed:?}, expected {expected:?}",
			directory.display()
		))
		.into());
	}
	for name in &observed {
		let path = directory.join(name);
		if fs::metadata(&path)?.len() == 0 {
			return Err(invalid(format!("artifact {} is empty", path.display())).into());
		}
	}
	Ok(())
}

fn rust_path_literal(path: &Path) -> AcceptanceResult<String> { Ok(format!("{:?}", path_text(path)?)) }

fn verify_one_cuda_image(kernels: &RealizedNativeKernelSet) -> AcceptanceResult<()> {
	let images = kernels.kernels();
	if images.len() != 1 {
		return Err(invalid(format!(
			"expected one bundled CUDA image, observed {}",
			images.len()
		))
		.into());
	}
	let image = &images[0];
	if image.format() != NativeKernelFormat::Cubin {
		return Err(invalid(format!(
			"CUDA acceptance realized a {:?} image",
			image.format()
		))
		.into());
	}
	if image.bytes().is_empty() || image.entries().is_empty() {
		return Err(invalid("bundled CUDA image or its logical entry table is empty").into());
	}
	Ok(())
}

fn verify_cuda_evidence(evidence: &NativeExecutionEvidence) -> AcceptanceResult<()> {
	if evidence.loop_realization_calls() != 0 {
		return Err(invalid(format!(
			"native loop performed {} forbidden realization calls",
			evidence.loop_realization_calls()
		))
		.into());
	}
	if !evidence.teardown_completed() || evidence.live_resources_after_teardown() != 0 {
		return Err(invalid(format!(
			"native teardown incomplete: completed={}, live_resources={}",
			evidence.teardown_completed(),
			evidence.live_resources_after_teardown()
		))
		.into());
	}
	let devices = evidence.devices();
	if devices.len() != 1 {
		return Err(invalid(format!(
			"expected evidence for one CUDA device, observed {}",
			devices.len()
		))
		.into());
	}
	let observed_device = &devices[0];
	if observed_device.backend != NativeBackendKind::Cuda || observed_device.image_loads != 1 {
		return Err(invalid(format!(
			"expected exactly one real CUDA image load, observed backend {:?} with {} loads",
			observed_device.backend, observed_device.image_loads
		))
		.into());
	}
	if observed_device.entry_lookups == 0
		|| observed_device.queues == 0
		|| observed_device.completion_objects == 0
		|| observed_device.persistent_allocations == 0
	{
		return Err(invalid(format!(
			"native CUDA evidence is incomplete: {observed_device:?}"
		))
		.into());
	}
	Ok(())
}

fn verify_completed_lifecycle(journal: &RunJournal, iterations: usize) -> AcceptanceResult<()> {
	let started = journal
		.logical_events()
		.iter()
		.filter(|event| matches!(event, LogicalEvent::LoopIterationStarted { .. }))
		.count();
	let completed = journal
		.logical_events()
		.iter()
		.filter(|event| matches!(event, LogicalEvent::LoopIterationCompleted { .. }))
		.count();
	if started != iterations || completed != iterations {
		return Err(invalid(format!(
			"expected {iterations} complete loop iterations, observed {started} starts and {completed} completions"
		))
		.into());
	}
	for required in [
		"prepared",
		"initialized",
		"loop-started",
		"loop-completed",
		"exited",
	] {
		let present = journal.logical_events().iter().any(|event| {
			match (required, event) {
				("prepared", LogicalEvent::Prepared { .. })
				| ("initialized", LogicalEvent::Initialized { .. })
				| ("loop-started", LogicalEvent::LoopStarted { .. })
				| ("loop-completed", LogicalEvent::LoopCompleted { .. })
				| ("exited", LogicalEvent::Exited { .. }) => true,
				_ => false,
			}
		});
		if !present {
			return Err(invalid(format!("native lifecycle omitted {required}")).into());
		}
	}
	let bind_count = journal
		.physical_calls()
		.iter()
		.filter(|call| matches!(call, PhysicalCall::BindResources))
		.count();
	let destroy_count = journal
		.physical_calls()
		.iter()
		.filter(|call| matches!(call, PhysicalCall::DestroyResources))
		.count();
	if bind_count != 1 || destroy_count != 1 {
		return Err(invalid(format!(
			"expected one native bind and destroy, observed {bind_count} binds and {destroy_count} destroys"
		))
		.into());
	}
	Ok(())
}

fn run_llama_gate(inputs: &Inputs, scratch: &Path, record: &mut AcceptanceRecord) -> AcceptanceResult<()> {
	let corpus = inputs.dataset_root.join("llamacpp-archs-seed42");
	let tokens = corpus.join("tokens.txt");
	let model = corpus.join("llama-dense.gguf");
	let reference = read_lgt0(&corpus.join("llama-dense.logits"))?;
	let token_count = fs::read_to_string(&tokens)?
		.lines()
		.filter(|line| !line.trim().is_empty())
		.count();
	if token_count != LLAMA_POSITIONS {
		return Err(invalid(format!(
			"expected {LLAMA_POSITIONS} token IDs, observed {token_count}"
		))
		.into());
	}

	let warm_recipe = run_recipe_llama(&tokens, &model)?;
	let mut maximum_recipe_nmse = verify_logits("Recipe warm-up", &warm_recipe.values, &reference)?;
	record.field("llama_native_image_loads", warm_recipe.native.image_loads);
	record.field(
		"llama_native_entry_lookups",
		warm_recipe.native.entry_lookups,
	);
	record.field("llama_native_queues", warm_recipe.native.queues);
	record.field(
		"llama_native_completion_objects",
		warm_recipe.native.completion_objects,
	);
	record.field(
		"llama_native_persistent_allocations",
		warm_recipe.native.persistent_allocations,
	);
	record.field(
		"llama_calculation_submissions",
		warm_recipe.native.calculation_submissions,
	);
	record.field(
		"llama_transfer_submissions",
		warm_recipe.native.transfer_submissions,
	);
	record.field(
		"llama_metric_submissions",
		warm_recipe.native.metric_submissions,
	);
	let warm_oracle_path = scratch.join("llama-oracle-warmup.logits");
	let warm_oracle = run_oracle(&inputs.llama_oracle, &model, &tokens, &warm_oracle_path)?;
	let mut maximum_oracle_nmse = verify_logits("llama.cpp warm-up", &warm_oracle.values, &reference)?;

	let mut recipe_samples = Vec::with_capacity(inputs.samples);
	let mut oracle_samples = Vec::with_capacity(inputs.samples);
	let mut sample_index = 0_usize;
	while recipe_samples.len() < inputs.samples || oracle_samples.len() < inputs.samples {
		for implementation in ["recipe", "oracle", "oracle", "recipe"] {
			match implementation {
				"recipe" if recipe_samples.len() < inputs.samples => {
					let run = run_recipe_llama(&tokens, &model)?;
					maximum_recipe_nmse = maximum_recipe_nmse.max(verify_logits(
						"Recipe measured run",
						&run.values,
						&reference,
					)?);
					recipe_samples.push(tokens_per_second(token_count, run.elapsed)?);
				}
				"oracle" if oracle_samples.len() < inputs.samples => {
					let output = scratch.join(format!("llama-oracle-{sample_index}.logits"));
					sample_index += 1;
					let run = run_oracle(&inputs.llama_oracle, &model, &tokens, &output)?;
					maximum_oracle_nmse = maximum_oracle_nmse.max(verify_logits(
						"llama.cpp measured run",
						&run.values,
						&reference,
					)?);
					oracle_samples.push(tokens_per_second(token_count, run.elapsed)?);
				}
				_ => {}
			}
		}
	}
	let recipe_median = median(&recipe_samples)?;
	let oracle_median = median(&oracle_samples)?;
	record.field("llama_positions", LLAMA_POSITIONS);
	record.field("llama_vocabulary", LLAMA_VOCABULARY);
	record.field("recipe_maximum_nmse", format!("{maximum_recipe_nmse:.12e}"));
	record.field(
		"llamacpp_maximum_nmse",
		format!("{maximum_oracle_nmse:.12e}"),
	);
	record.field("recipe_tok_s_samples", format!("{recipe_samples:?}"));
	record.field("llamacpp_tok_s_samples", format!("{oracle_samples:?}"));
	record.field("recipe_median_tok_s", format!("{recipe_median:.9}"));
	record.field("llamacpp_median_tok_s", format!("{oracle_median:.9}"));
	println!(
		"acceptance\tworkload\tllama-gguf\trecipe_tok_s\t{recipe_samples:?}\tllamacpp_tok_s\t{oracle_samples:?}"
	);
	if recipe_median < oracle_median {
		return Err(invalid(format!(
			"Recipe median {recipe_median:.6} tok/s is slower than llama.cpp median {oracle_median:.6} tok/s"
		))
		.into());
	}
	println!(
		"acceptance\tworkload\tllama-gguf\trecipe_median_tok_s\t{recipe_median:.6}\tllamacpp_median_tok_s\t{oracle_median:.6}\tresult\tpass"
	);
	Ok(())
}

fn run_recipe_llama(tokens: &Path, model: &Path) -> AcceptanceResult<TimedLogits> {
	recipe.data(path_text(tokens)?);
	let model_path = path_text(model)?;
	recipe.model().load(&model_path);
	let report = recipe.infer().log([Time, Device]).evaluate()?;
	if report.kind() != InferenceModelKind::GgufLlama {
		return Err(invalid("GGUF acceptance did not dispatch the Llama inference path").into());
	}
	if report.devices().len() != 1 {
		return Err(invalid(format!(
			"CUDA acceptance requires exactly one participating GPU, observed {:?}",
			report.devices()
		))
		.into());
	}
	verify_completed_lifecycle(report.journal(), 1)?;
	verify_one_cuda_image(
		report.native_kernels()
			.ok_or_else(|| invalid("GGUF inference omitted its native image evidence"))?,
	)?;
	verify_cuda_evidence(report.native_evidence())?;
	let observed_device = &report.native_evidence().devices()[0];
	let calculation_submissions = report
		.journal()
		.physical_calls()
		.iter()
		.filter(|call| matches!(call, PhysicalCall::SubmitCalculation { .. }))
		.count();
	let transfer_submissions = report
		.journal()
		.physical_calls()
		.iter()
		.filter(|call| {
			matches!(
				call,
				PhysicalCall::SubmitInternalTransfer { .. }
					| PhysicalCall::SubmitExitTransfer { .. }
					| PhysicalCall::SubmitExternalIngress { .. }
					| PhysicalCall::SubmitExternalEgress { .. }
			)
		})
		.count();
	let metric_submissions = report
		.journal()
		.physical_calls()
		.iter()
		.filter(|call| matches!(call, PhysicalCall::SubmitMetric { .. }))
		.count();
	Ok(TimedLogits {
		elapsed: report.elapsed(),
		values: report.values().collect(),
		native: NativeRunRecord {
			image_loads: observed_device.image_loads,
			entry_lookups: observed_device.entry_lookups,
			queues: observed_device.queues,
			completion_objects: observed_device.completion_objects,
			persistent_allocations: observed_device.persistent_allocations,
			calculation_submissions,
			transfer_submissions,
			metric_submissions,
		},
	})
}

fn run_oracle(oracle: &Path, model: &Path, tokens: &Path, output: &Path) -> AcceptanceResult<TimedLogits> {
	let process = Command::new(oracle)
		.arg("--model")
		.arg(model)
		.arg("--tokens")
		.arg(tokens)
		.arg("--output")
		.arg(output)
		.output()?;
	if !process.status.success() {
		return Err(invalid(format!(
			"llama.cpp oracle failed with {}: {}",
			process.status,
			String::from_utf8_lossy(&process.stderr)
		))
		.into());
	}
	let stdout = String::from_utf8(process.stdout)?;
	let elapsed_ns = stdout
		.lines()
		.find_map(|line| line.strip_prefix("elapsed_ns="))
		.ok_or_else(|| invalid("llama.cpp oracle omitted elapsed_ns"))?
		.parse::<u64>()?;
	if elapsed_ns == 0 {
		return Err(invalid("llama.cpp oracle reported zero elapsed time").into());
	}
	Ok(TimedLogits {
		elapsed: Duration::from_nanos(elapsed_ns),
		values: read_lgt0(output)?,
		native: NativeRunRecord {
			image_loads: 0,
			entry_lookups: 0,
			queues: 0,
			completion_objects: 0,
			persistent_allocations: 0,
			calculation_submissions: 0,
			transfer_submissions: 0,
			metric_submissions: 0,
		},
	})
}

fn verify_logits(label: &str, actual: &[f32], expected: &[f32]) -> AcceptanceResult<f64> {
	if actual.len() != LLAMA_POSITIONS * LLAMA_VOCABULARY || actual.len() != expected.len() {
		return Err(invalid(format!(
			"{label} returned {} logits; expected {}",
			actual.len(),
			expected.len()
		))
		.into());
	}
	let mut squared_error = 0.0_f64;
	let mut reference_energy = 0.0_f64;
	for (&actual, &expected) in actual.iter().zip(expected) {
		let error = f64::from(actual) - f64::from(expected);
		squared_error += error * error;
		reference_energy += f64::from(expected) * f64::from(expected);
	}
	let nmse = squared_error / reference_energy;
	if !nmse.is_finite() || nmse >= MAXIMUM_LLAMA_NMSE {
		return Err(invalid(format!(
			"{label} logits failed correctness with NMSE {nmse:.9e}"
		))
		.into());
	}
	Ok(nmse)
}

fn read_lgt0(path: &Path) -> AcceptanceResult<Vec<f32>> {
	let bytes = fs::read(path)?;
	if bytes.get(..4) != Some(b"LGT0") || bytes.len() < 12 {
		return Err(invalid(format!("{} is not an LGT0 file", path.display())).into());
	}
	let positions = u32::from_le_bytes(bytes[4..8].try_into()?) as usize;
	let vocabulary = u32::from_le_bytes(bytes[8..12].try_into()?) as usize;
	if positions != LLAMA_POSITIONS || vocabulary != LLAMA_VOCABULARY {
		return Err(invalid(format!(
			"{} has shape [{positions}, {vocabulary}], expected [{LLAMA_POSITIONS}, {LLAMA_VOCABULARY}]",
			path.display()
		))
		.into());
	}
	let values = bytes[12..]
		.chunks_exact(4)
		.map(|chunk| <[u8; 4]>::try_from(chunk).map(f32::from_le_bytes))
		.collect::<Result<Vec<_>, _>>()?;
	if values.len() != positions * vocabulary || bytes.len() != 12 + values.len() * 4 {
		return Err(invalid(format!(
			"{} has an inconsistent LGT0 payload",
			path.display()
		))
		.into());
	}
	Ok(values)
}

fn tokens_per_second(tokens: usize, elapsed: Duration) -> AcceptanceResult<f64> {
	let seconds = elapsed.as_secs_f64();
	if seconds <= 0.0 || !seconds.is_finite() {
		return Err(invalid("measured inference duration is not positive and finite").into());
	}
	Ok(tokens as f64 / seconds)
}

fn median(samples: &[f64]) -> AcceptanceResult<f64> {
	if samples.len() != REQUIRED_SAMPLES
		|| samples
			.iter()
			.any(|sample| !sample.is_finite() || *sample <= 0.0)
	{
		return Err(invalid("throughput sample set is incomplete or invalid").into());
	}
	let mut ordered = samples.to_vec();
	ordered.sort_by(f64::total_cmp);
	Ok(ordered[ordered.len() / 2])
}

fn path_text(path: &Path) -> AcceptanceResult<String> {
	path.to_str()
		.map(ToOwned::to_owned)
		.ok_or_else(|| invalid(format!("path is not valid UTF-8: {}", path.display())).into())
}
