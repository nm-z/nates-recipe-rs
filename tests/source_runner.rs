#![allow(unsafe_code)]

use std::io::{BufRead as _, Read as _};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use recipe_training::{
	CheckpointArtifact, CheckpointBlockImage, CheckpointDecodeLimits, CheckpointLayerImage, decode_checkpoint,
};

static FIXTURE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct FixtureDirectory(PathBuf);

impl FixtureDirectory {
	fn new(name: &str) -> Self {
		let sequence = FIXTURE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
		let path = std::env::temp_dir().join(format!(
			"recipe-source-runner-{name}-{}-{sequence}",
			std::process::id()
		));
		std::fs::create_dir(&path).expect("create source-runner fixture directory");
		Self(path)
	}

	fn path(&self) -> &Path {
		&self.0
	}
}

impl Drop for FixtureDirectory {
	fn drop(&mut self) {
		let _ = std::fs::remove_dir_all(&self.0);
	}
}

fn run_native_training_until_sigint(source: &Path, cache: &Path) -> (std::process::ExitStatus, String, String) {
	let mut child = Command::new(env!("CARGO_BIN_EXE_recipe"))
		.arg("run")
		.arg(source)
		.env("XDG_CACHE_HOME", cache)
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn()
		.expect("start native Recipe training through source runner");
	let stdout = child.stdout.take().expect("capture Recipe stdout");
	let stderr = child.stderr.take().expect("capture Recipe stderr");
	let (first_line_sender, first_line_receiver) = mpsc::sync_channel(1);
	let stdout_reader = std::thread::spawn(move || {
		let mut reader = std::io::BufReader::new(stdout);
		let mut first_line = String::new();
		let result = reader
			.read_line(&mut first_line)
			.map(|_| first_line.clone());
		let _ = first_line_sender.send(result);
		let mut remainder = Vec::new();
		reader.read_to_end(&mut remainder)
			.expect("read remaining native Recipe stdout");
		(first_line, remainder)
	});
	let stderr_reader = std::thread::spawn(move || {
		let mut bytes = Vec::new();
		std::io::BufReader::new(stderr)
			.read_to_end(&mut bytes)
			.expect("read native Recipe stderr");
		bytes
	});

	let first_line = match first_line_receiver.recv_timeout(Duration::from_secs(120)) {
		Ok(Ok(line)) => line,
		Ok(Err(error)) => {
			let _ = child.kill();
			let _ = child.wait();
			panic!("read first native epoch line: {error}");
		}
		Err(error) => {
			let _ = child.kill();
			let _ = child.wait();
			panic!("native training did not report progress within 120 seconds: {error}");
		}
	};
	assert!(
		first_line.contains("epoch"),
		"first native output: {first_line:?}"
	);
	// SAFETY: `child.id()` belongs to the test-owned Recipe parent. The signal
	// requests its documented graceful-stop path without dereferencing memory.
	let signal_result = unsafe { libc::kill(child.id() as libc::pid_t, libc::SIGINT) };
	assert_eq!(
		signal_result,
		0,
		"send SIGINT to native Recipe parent: {}",
		std::io::Error::last_os_error()
	);

	let deadline = Instant::now() + Duration::from_secs(120);
	let status = loop {
		match child
			.try_wait()
			.expect("inspect native Recipe source runner")
		{
			Some(status) => break status,
			None if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(10)),
			None => {
				let _ = child.kill();
				let _ = child.wait();
				panic!("native Recipe source runner did not stop within 120 seconds");
			}
		}
	};
	let (captured_first_line, stdout_remainder) = stdout_reader
		.join()
		.expect("join native Recipe stdout reader");
	let stderr = stderr_reader
		.join()
		.expect("join native Recipe stderr reader");
	let stdout = format!(
		"{captured_first_line}{}",
		String::from_utf8_lossy(&stdout_remainder)
	);
	(
		status,
		stdout,
		String::from_utf8_lossy(&stderr).into_owned(),
	)
}

fn first_dense_layer(artifact: &CheckpointArtifact) -> &CheckpointLayerImage {
	artifact
		.blocks()
		.iter()
		.find_map(|block| match block {
			CheckpointBlockImage::Layer(layer) => Some(layer),
			CheckpointBlockImage::Embedding(_)
			| CheckpointBlockImage::Attention(_)
			| CheckpointBlockImage::Rnn(_)
			| CheckpointBlockImage::Gru(_)
			| CheckpointBlockImage::Lstm(_)
			| CheckpointBlockImage::Convolution(_)
			| CheckpointBlockImage::Pool(_)
			| CheckpointBlockImage::KMeans(_)
			| CheckpointBlockImage::Tree(_)
			| CheckpointBlockImage::Residual(_) => None,
		})
		.expect("saved model contains a dense layer block")
}

#[test]
fn source_runner_rejects_an_ignored_training_result() {
	let fixture = FixtureDirectory::new("ignored-training-result");
	let source = fixture.path().join("train.rs");
	std::fs::write(
		&source,
		r#"use recipe::*;

	fn main() {
		recipe.data("missing.csv").target("target").norm(z_score).split(0.8);
		recipe.model().layer(1).loss(bce);
		recipe.train().optimizer(adamw).epochs(1).lr(0.001).cos().run();
	}
"#,
	)
	.expect("write ignored-result source fixture");

	let output = Command::new(env!("CARGO_BIN_EXE_recipe"))
		.arg("run")
		.arg(&source)
		.env("XDG_CACHE_HOME", fixture.path().join("cache"))
		.output()
		.expect("run ignored-result source fixture");
	assert!(!output.status.success());
	let stderr = String::from_utf8_lossy(&output.stderr);
	assert!(
		stderr.contains("unused `Result` that must be used"),
		"stderr: {stderr}"
	);
	assert!(!stderr.contains("panicked at"), "stderr: {stderr}");
}

#[test]
fn expected_training_failure_is_typed_plaintext_and_nonzero() {
	let fixture = FixtureDirectory::new("typed-training-error");
	let missing = fixture.path().join("missing.csv");
	let source = fixture.path().join("train.rs");
	std::fs::write(
		&source,
		format!(
			r#"use recipe::*;

	fn main() -> TrainingResult<()> {{
		recipe.data({missing:?}).target("target").norm(z_score).split(0.8);
		recipe.model().layer(1).loss(bce);
		recipe.train().optimizer(adamw).epochs(1).lr(0.001).cos().run()?;
		Ok(())
	}}
"#,
			missing = missing.to_str().expect("missing fixture path is UTF-8"),
		),
	)
	.expect("write typed-error source fixture");

	let output = Command::new(env!("CARGO_BIN_EXE_recipe"))
		.arg("run")
		.arg(&source)
		.env("XDG_CACHE_HOME", fixture.path().join("cache"))
		.output()
		.expect("run typed-error source fixture");
	assert!(!output.status.success());
	let stderr = String::from_utf8_lossy(&output.stderr);
	assert!(stderr.contains("prepare training data"), "stderr: {stderr}");
	assert!(stderr.contains("exited with code 1"), "stderr: {stderr}");
	assert!(!stderr.contains("panicked at"), "stderr: {stderr}");
	assert!(!stderr.contains("recipe_run"), "stderr: {stderr}");
}

#[test]
fn epoch_output_is_observable_before_the_source_child_exits() {
	let fixture = FixtureDirectory::new("live-output");
	let source = fixture.path().join("train.rs");
	let release = fixture.path().join("release-child");
	std::fs::write(
		&source,
		r#"use std::io::Write as _;
	use std::path::Path;
	use std::time::Duration;

	fn main() {
		let release = std::env::args_os().nth(1).expect("release path");
		println!("epoch 1  loss 0.1250");
		std::io::stdout().flush().expect("flush epoch output");
		while !Path::new(&release).exists() {
			std::thread::sleep(Duration::from_millis(10));
		}
		println!("training child exited");
	}
"#,
	)
	.expect("write live-output source fixture");

	let mut child = Command::new(env!("CARGO_BIN_EXE_recipe"))
		.arg("run")
		.arg(&source)
		.arg(&release)
		.env("XDG_CACHE_HOME", fixture.path().join("cache"))
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn()
		.expect("start Recipe source runner");
	let stdout = child.stdout.take().expect("capture Recipe stdout");
	let stderr = child.stderr.take().expect("capture Recipe stderr");
	let (first_line_sender, first_line_receiver) = mpsc::sync_channel(1);
	let stdout_reader = std::thread::spawn(move || {
		let mut reader = std::io::BufReader::new(stdout);
		let mut first_line = String::new();
		let first_line_result = reader
			.read_line(&mut first_line)
			.map(|_| first_line.clone());
		let _ = first_line_sender.send(first_line_result);
		let mut remainder = Vec::new();
		reader.read_to_end(&mut remainder)
			.expect("read remaining Recipe stdout");
		(first_line, remainder)
	});
	let stderr_reader = std::thread::spawn(move || {
		let mut bytes = Vec::new();
		std::io::BufReader::new(stderr)
			.read_to_end(&mut bytes)
			.expect("read Recipe stderr");
		bytes
	});

	let first_line = first_line_receiver.recv_timeout(Duration::from_secs(30));
	let child_was_running = first_line
		.as_ref()
		.ok()
		.and_then(|line| line.as_ref().ok())
		.is_some_and(|_| {
			child.try_wait()
				.expect("inspect Recipe source runner")
				.is_none()
		});
	std::fs::write(&release, b"release").expect("release source child");
	let status = child.wait().expect("wait for Recipe source runner");
	let (captured_first_line, stdout_remainder) = stdout_reader.join().expect("join Recipe stdout reader");
	let stderr = stderr_reader.join().expect("join Recipe stderr reader");
	let complete_stdout = format!(
		"{captured_first_line}{}",
		String::from_utf8_lossy(&stdout_remainder)
	);
	let stderr = String::from_utf8_lossy(&stderr);

	let first_line = first_line
		.expect("epoch output was not forwarded within 30 seconds")
		.expect("read first forwarded epoch line");
	assert_eq!(
		first_line, "epoch 1  loss 0.1250\n",
		"stdout: {complete_stdout}\nstderr: {stderr}"
	);
	assert!(
		child_was_running,
		"Recipe or its source child exited before the epoch line was observed\nstdout: {complete_stdout}\nstderr: {stderr}"
	);
	assert!(
		status.success(),
		"stdout: {complete_stdout}\nstderr: {stderr}"
	);
	assert!(complete_stdout.contains("training child exited"));
}

#[test]
fn forwarding_failure_makes_the_source_runner_fail() {
	let fixture = FixtureDirectory::new("closed-output");
	let source = fixture.path().join("train.rs");
	std::fs::write(
		&source,
		r#"use std::io::Write as _;

	fn main() {
		println!("this output must be forwarded");
		std::io::stdout().flush().expect("flush source output");
	}
"#,
	)
	.expect("write closed-output source fixture");

	let mut child = Command::new(env!("CARGO_BIN_EXE_recipe"))
		.arg("run")
		.arg(&source)
		.env("XDG_CACHE_HOME", fixture.path().join("cache"))
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn()
		.expect("start Recipe source runner");
	drop(child
		.stdout
		.take()
		.expect("capture and close Recipe stdout"));
	let mut stderr = Vec::new();
	child.stderr
		.take()
		.expect("capture Recipe stderr")
		.read_to_end(&mut stderr)
		.expect("read Recipe stderr");
	let status = child.wait().expect("wait for Recipe source runner");
	let stderr = String::from_utf8_lossy(&stderr);

	assert!(
		!status.success(),
		"forwarding to a closed stdout pipe unexpectedly succeeded"
	);
	assert!(stderr.contains("write training stdout"), "stderr: {stderr}");
}

#[test]
fn first_sigint_reaches_the_source_child_and_the_parent_cleans_up() {
	let fixture = FixtureDirectory::new("graceful-sigint");
	let source = fixture.path().join("train.rs");
	std::fs::write(
		&source,
		r#"use std::io::Write as _;
	use std::sync::atomic::{AtomicBool, Ordering};
	use std::time::Duration;

	static STOP_REQUESTED: AtomicBool = AtomicBool::new(false);

	extern "C" fn request_stop(_signal: i32) {
		STOP_REQUESTED.store(true, Ordering::Release);
	}

	unsafe extern "C" {
		fn signal(signal: i32, handler: usize) -> usize;
	}

	fn main() {
		unsafe {
			signal(2, request_stop as *const () as usize);
		}
		println!("training child ready");
		std::io::stdout().flush().expect("flush readiness");
		while !STOP_REQUESTED.load(Ordering::Acquire) {
			std::thread::sleep(Duration::from_millis(10));
		}
		println!("training child stopped cleanly");
		std::io::stdout().flush().expect("flush stop confirmation");
	}
"#,
	)
	.expect("write SIGINT source fixture");

	let cache = fixture.path().join("cache");
	let mut child = Command::new(env!("CARGO_BIN_EXE_recipe"))
		.arg("run")
		.arg(&source)
		.env("XDG_CACHE_HOME", &cache)
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn()
		.expect("start Recipe source runner");
	let stdout = child.stdout.take().expect("capture Recipe stdout");
	let stderr = child.stderr.take().expect("capture Recipe stderr");
	let (first_line_sender, first_line_receiver) = mpsc::sync_channel(1);
	let stdout_reader = std::thread::spawn(move || {
		let mut reader = std::io::BufReader::new(stdout);
		let mut first_line = String::new();
		let result = reader
			.read_line(&mut first_line)
			.map(|_| first_line.clone());
		let _ = first_line_sender.send(result);
		let mut remainder = Vec::new();
		reader.read_to_end(&mut remainder)
			.expect("read remaining Recipe stdout");
		(first_line, remainder)
	});
	let stderr_reader = std::thread::spawn(move || {
		let mut bytes = Vec::new();
		std::io::BufReader::new(stderr)
			.read_to_end(&mut bytes)
			.expect("read Recipe stderr");
		bytes
	});

	let first_line = match first_line_receiver.recv_timeout(Duration::from_secs(30)) {
		Ok(Ok(line)) => line,
		Ok(Err(error)) => {
			let _ = child.kill();
			let _ = child.wait();
			panic!("read first child line: {error}");
		}
		Err(error) => {
			let _ = child.kill();
			let _ = child.wait();
			panic!("training child did not become ready within 30 seconds: {error}");
		}
	};
	assert_eq!(first_line, "training child ready\n");
	// SAFETY: `child.id()` is the positive process id returned by spawning this
	// test-owned process. Sending SIGINT does not dereference process memory.
	let signal_result = unsafe { libc::kill(child.id() as libc::pid_t, libc::SIGINT) };
	assert_eq!(
		signal_result,
		0,
		"send SIGINT to Recipe source-runner parent: {}",
		std::io::Error::last_os_error()
	);

	let deadline = Instant::now() + Duration::from_secs(30);
	let status = loop {
		match child.try_wait().expect("inspect Recipe source runner") {
			Some(status) => break status,
			None if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(10)),
			None => {
				let _ = child.kill();
				let _ = child.wait();
				panic!("Recipe source runner did not finish graceful SIGINT handling within 30 seconds");
			}
		}
	};
	let (captured_first_line, stdout_remainder) = stdout_reader.join().expect("join Recipe stdout reader");
	let stderr = stderr_reader.join().expect("join Recipe stderr reader");
	let complete_stdout = format!(
		"{captured_first_line}{}",
		String::from_utf8_lossy(&stdout_remainder)
	);
	let stderr = String::from_utf8_lossy(&stderr);

	assert!(
		status.success(),
		"stdout: {complete_stdout}\nstderr: {stderr}"
	);
	assert!(
		complete_stdout.contains("training child stopped cleanly"),
		"stdout: {complete_stdout}\nstderr: {stderr}"
	);
	let run_directory = cache.join("recipe-next/run");
	let leftovers = std::fs::read_dir(&run_directory)
		.expect("inspect private source-runner directory")
		.collect::<Result<Vec<_>, _>>()
		.expect("read private source-runner entries");
	assert!(
		leftovers.is_empty(),
		"compiled source-runner artifacts remained in {}: {leftovers:?}",
		run_directory.display()
	);
}

#[test]
#[ignore = "requires current measured CUDA/HSA hardware and offline artifact toolchains"]
fn sigint_saves_a_loadable_latest_completed_training_state() {
	let fixture = FixtureDirectory::new("native-graceful-save");
	let cache = fixture.path().join("cache");
	let probe = Command::new(env!("CARGO_BIN_EXE_recipe"))
		.arg("probe")
		.env("XDG_CACHE_HOME", &cache)
		.output()
		.expect("measure native hardware for graceful-save acceptance");
	assert!(
		probe.status.success(),
		"probe stdout: {}\nprobe stderr: {}",
		String::from_utf8_lossy(&probe.stdout),
		String::from_utf8_lossy(&probe.stderr)
	);

	let dataset = fixture.path().join("train.csv");
	std::fs::write(&dataset, "feature,target\n0,0\n1,1\n2,0\n4,1\n8,1\n16,0\n")
		.expect("write native graceful-save dataset");
	let model = fixture.path().join("interrupted.ogdl");
	let kernel = fixture.path().join("interrupted.hsaco");
	let source = fixture.path().join("train.rs");
	let source_text = format!(
		r#"use recipe::*;

	fn main() -> TrainingResult<()> {{
		recipe.data({dataset:?}).target("target").norm(z_score).split(0.8);
		recipe.model().layer(1).loss(bce);
		recipe
			.train()
			.optimizer(adamw)
			.lr(0.0003)
			.log(Loss)
			.every(1_000)
			.save({model:?}, {kernel:?})
			.run()?;
		Ok(())
	}}
"#,
		dataset = dataset.to_str().expect("fixture dataset path is UTF-8"),
		model = model.to_str().expect("fixture model path is UTF-8"),
		kernel = kernel.to_str().expect("fixture kernel path is UTF-8"),
	);
	std::fs::write(&source, source_text).expect("write native graceful-save source");

	let (status, stdout, stderr) = run_native_training_until_sigint(&source, &cache);
	assert!(status.success(), "stdout: {stdout}\nstderr: {stderr}");

	let bytes = std::fs::read(&model).expect("read SIGINT-saved model");
	let artifact = decode_checkpoint(&bytes, CheckpointDecodeLimits::default())
		.expect("decode and validate SIGINT-saved model");
	let layer = first_dense_layer(&artifact);
	let completed_state = layer
		.weight()
		.first_moment()
		.bytes()
		.iter()
		.chain(layer.bias().first_moment().bytes())
		.any(|byte| *byte != 0);
	assert!(
		completed_state,
		"SIGINT-saved model contains only initial zero optimizer moments"
	);
	let kernel_bytes = std::fs::read(&kernel).expect("read SIGINT-saved native kernel");
	assert!(
		kernel_bytes.len() >= 64,
		"saved HSACO is shorter than an ELF header"
	);
	assert_eq!(
		&kernel_bytes[..4],
		b"\x7fELF",
		"saved native kernel is not ELF"
	);
	assert_eq!(
		kernel_bytes[7], 0x40,
		"saved native kernel is not AMD HSA ELF"
	);
	assert_eq!(
		u16::from_le_bytes([kernel_bytes[18], kernel_bytes[19]]),
		224,
		"saved native kernel is not an AMDGPU code object"
	);
	let run_directory = cache.join("recipe-next/run");
	let leftovers = std::fs::read_dir(&run_directory)
		.expect("inspect private native source-runner directory")
		.collect::<Result<Vec<_>, _>>()
		.expect("read private native source-runner entries");
	assert!(
		leftovers.is_empty(),
		"compiled native source-runner artifacts remained in {}: {leftovers:?}",
		run_directory.display()
	);
}

#[test]
#[ignore = "requires current measured CUDA/HSA hardware and offline artifact toolchains"]
fn sigint_without_a_save_declaration_exports_nothing() {
	let fixture = FixtureDirectory::new("native-graceful-no-save");
	let cache = fixture.path().join("cache");
	let probe = Command::new(env!("CARGO_BIN_EXE_recipe"))
		.arg("probe")
		.env("XDG_CACHE_HOME", &cache)
		.output()
		.expect("measure native hardware for graceful no-save acceptance");
	assert!(
		probe.status.success(),
		"probe stdout: {}\nprobe stderr: {}",
		String::from_utf8_lossy(&probe.stdout),
		String::from_utf8_lossy(&probe.stderr)
	);

	let dataset = fixture.path().join("train.csv");
	std::fs::write(&dataset, "feature,target\n0,0\n1,1\n2,0\n4,1\n8,1\n16,0\n")
		.expect("write native graceful no-save dataset");
	let source = fixture.path().join("train.rs");
	let source_text = format!(
		r#"use recipe::*;

	fn main() -> TrainingResult<()> {{
		recipe.data({dataset:?}).target("target").norm(z_score).split(0.8);
		recipe.model().layer(1).loss(bce);
		recipe
			.train()
			.optimizer(adamw)
			.lr(0.0003)
			.log(Loss)
			.every(1)
			.run()?;
		Ok(())
	}}
"#,
		dataset = dataset.to_str().expect("fixture dataset path is UTF-8"),
	);
	std::fs::write(&source, source_text).expect("write native graceful no-save source");

	let (status, stdout, stderr) = run_native_training_until_sigint(&source, &cache);
	assert!(status.success(), "stdout: {stdout}\nstderr: {stderr}");
	let exported = std::fs::read_dir(fixture.path())
		.expect("inspect no-save fixture")
		.collect::<Result<Vec<_>, _>>()
		.expect("read no-save fixture entries")
		.into_iter()
		.map(|entry| entry.path())
		.filter(|path| {
			matches!(
				path.extension().and_then(|extension| extension.to_str()),
				Some("ogdl" | "cubin" | "hsaco")
			)
		})
		.collect::<Vec<_>>();
	assert!(
		exported.is_empty(),
		"training without `.save(...)` exported user-owned artifacts: {exported:?}"
	);
	let run_directory = cache.join("recipe-next/run");
	let leftovers = std::fs::read_dir(&run_directory)
		.expect("inspect private native source-runner directory")
		.collect::<Result<Vec<_>, _>>()
		.expect("read private native source-runner entries");
	assert!(
		leftovers.is_empty(),
		"compiled native source-runner artifacts remained in {}: {leftovers:?}",
		run_directory.display()
	);
}

#[test]
#[ignore = "requires current measured CUDA/HSA hardware and offline artifact toolchains"]
fn one_path_save_routes_model_only_and_kernel_only_without_companion_artifacts() {
	let fixture = FixtureDirectory::new("native-one-path-save");
	let cache = fixture.path().join("cache");
	let probe = Command::new(env!("CARGO_BIN_EXE_recipe"))
		.arg("probe")
		.env("XDG_CACHE_HOME", &cache)
		.output()
		.expect("measure native hardware for one-path save acceptance");
	assert!(
		probe.status.success(),
		"probe stdout: {}\nprobe stderr: {}",
		String::from_utf8_lossy(&probe.stdout),
		String::from_utf8_lossy(&probe.stderr)
	);
	let dataset = fixture.path().join("train.csv");
	std::fs::write(&dataset, "feature,target\n0,0\n1,1\n2,0\n4,1\n8,1\n16,0\n").expect("write one-path save dataset");
	let model = fixture.path().join("model-only.ogdl");
	let kernel = fixture.path().join("kernel-only.hsaco");
	let source_text = |save: &Path| {
		format!(
			r#"use recipe::*;

	fn main() -> TrainingResult<()> {{
		recipe.data({dataset:?}).target("target").norm(z_score).split(0.8);
		recipe.model().layer(1).loss(bce);
		recipe
			.train()
			.optimizer(adamw)
			.epochs(1)
			.lr(0.0003)
			.cos()
			.save({save:?})
			.run()?;
		Ok(())
	}}
"#,
			dataset = dataset.to_str().expect("dataset path is UTF-8"),
			save = save.to_str().expect("save path is UTF-8"),
		)
	};
	for (name, save) in [("model-only.rs", &model), ("kernel-only.rs", &kernel)] {
		let source = fixture.path().join(name);
		std::fs::write(&source, source_text(save)).expect("write one-path training source");
		let output = Command::new(env!("CARGO_BIN_EXE_recipe"))
			.arg("run")
			.arg(&source)
			.env("XDG_CACHE_HOME", &cache)
			.output()
			.expect("run one-path native training source");
		assert!(
			output.status.success(),
			"source: {}\nstdout: {}\nstderr: {}",
			source.display(),
			String::from_utf8_lossy(&output.stdout),
			String::from_utf8_lossy(&output.stderr)
		);
	}
	decode_checkpoint(
		&std::fs::read(&model).expect("read model-only save"),
		CheckpointDecodeLimits::default(),
	)
	.expect("decode model-only save");
	let kernel_bytes = std::fs::read(&kernel).expect("read kernel-only save");
	assert_eq!(&kernel_bytes[..4], b"\x7fELF");
	let artifacts = std::fs::read_dir(fixture.path())
		.expect("inspect one-path fixture")
		.collect::<Result<Vec<_>, _>>()
		.expect("read one-path fixture")
		.into_iter()
		.map(|entry| entry.path())
		.filter(|path| {
			matches!(
				path.extension().and_then(|extension| extension.to_str()),
				Some("ogdl" | "cubin" | "hsaco")
			)
		})
		.collect::<Vec<_>>();
	assert_eq!(
		artifacts.len(),
		2,
		"one-path saves invented companion artifacts: {artifacts:?}"
	);
}

#[test]
#[ignore = "requires current measured CUDA/HSA hardware and offline artifact toolchains"]
fn existing_model_and_kernel_resume_continues_state_and_reuses_exact_native_bytes() {
	let fixture = FixtureDirectory::new("native-existing-resume");
	let cache = fixture.path().join("cache");
	let probe = Command::new(env!("CARGO_BIN_EXE_recipe"))
		.arg("probe")
		.env("XDG_CACHE_HOME", &cache)
		.output()
		.expect("measure native hardware for existing-resume acceptance");
	assert!(
		probe.status.success(),
		"probe stdout: {}\nprobe stderr: {}",
		String::from_utf8_lossy(&probe.stdout),
		String::from_utf8_lossy(&probe.stderr)
	);

	let dataset = fixture.path().join("train.csv");
	std::fs::write(&dataset, "feature,target\n0,0\n1,1\n2,0\n4,1\n8,1\n16,0\n")
		.expect("write existing-resume dataset");
	let initial_model = fixture.path().join("initial.ogdl");
	let initial_kernel = fixture.path().join("initial.hsaco");
	let model_only_model = fixture.path().join("model-only.ogdl");
	let model_only_kernel = fixture.path().join("model-only.hsaco");
	let continued_model = fixture.path().join("continued.ogdl");
	let continued_kernel = fixture.path().join("continued.hsaco");
	let missing_resume_kernel = fixture.path().join("missing.hsaco");
	let missing_fallback_model = fixture.path().join("missing-fallback.ogdl");
	let missing_fallback_kernel = fixture.path().join("missing-fallback.hsaco");
	let initial_source = fixture.path().join("initial.rs");
	let resume_without_save_source = fixture.path().join("resume-without-save.rs");
	let model_only_source = fixture.path().join("model-only.rs");
	let continued_source = fixture.path().join("continued.rs");
	let missing_fallback_source = fixture.path().join("missing-fallback.rs");
	let training_source = |resume: Option<(&Path, Option<&Path>)>, save_model: &Path, save_kernel: &Path| {
		let resume = resume.map_or_else(String::new, |(model, kernel)| {
			let model = model.to_str().expect("resume model path is UTF-8");
			match kernel {
				Some(kernel) => format!(
					"\n\t\t\t.resume({model:?}, {:?})",
					kernel.to_str().expect("resume kernel path is UTF-8")
				),
				None => format!("\n\t\t\t.resume({model:?})"),
			}
		});
		format!(
			r#"use recipe::*;

	fn main() -> TrainingResult<()> {{
		recipe.data({dataset:?}).target("target").norm(z_score).split(0.8);
		recipe.model().layer(1).loss(bce);
		recipe
			.train()
			.optimizer(adamw)
			.epochs(1)
			.lr(0.0003)
			.cos(){resume}
			.save({save_model:?}, {save_kernel:?})
			.run()?;
		Ok(())
	}}
"#,
			dataset = dataset.to_str().expect("dataset path is UTF-8"),
			save_model = save_model.to_str().expect("save model path is UTF-8"),
			save_kernel = save_kernel.to_str().expect("save kernel path is UTF-8"),
		)
	};
	std::fs::write(
		&initial_source,
		training_source(None, &initial_model, &initial_kernel),
	)
	.expect("write initial training source");
	std::fs::write(
		&resume_without_save_source,
		format!(
			r#"use recipe::*;

	fn main() -> TrainingResult<()> {{
		recipe.data({dataset:?}).target("target").norm(z_score).split(0.8);
		recipe.model().layer(1).loss(bce);
		recipe
			.train()
			.optimizer(adamw)
			.epochs(1)
			.lr(0.0003)
			.cos()
			.resume({resume_model:?})
			.run()?;
		Ok(())
	}}
"#,
			dataset = dataset.to_str().expect("dataset path is UTF-8"),
			resume_model = initial_model.to_str().expect("resume model path is UTF-8"),
		),
	)
	.expect("write resume-without-save source");
	std::fs::write(
		&model_only_source,
		training_source(
			Some((&initial_model, None)),
			&model_only_model,
			&model_only_kernel,
		),
	)
	.expect("write model-only resume source");
	std::fs::write(
		&continued_source,
		training_source(
			Some((&initial_model, Some(&initial_kernel))),
			&continued_model,
			&continued_kernel,
		),
	)
	.expect("write continued training source");
	std::fs::write(
		&missing_fallback_source,
		training_source(
			Some((&initial_model, Some(&missing_resume_kernel))),
			&missing_fallback_model,
			&missing_fallback_kernel,
		),
	)
	.expect("write missing-kernel fallback source");

	let run_source = |source: &Path| {
		let output = Command::new(env!("CARGO_BIN_EXE_recipe"))
			.arg("run")
			.arg(source)
			.env("XDG_CACHE_HOME", &cache)
			.output()
			.expect("run native Recipe training source");
		assert!(
			output.status.success(),
			"source: {}\nstdout: {}\nstderr: {}",
			source.display(),
			String::from_utf8_lossy(&output.stdout),
			String::from_utf8_lossy(&output.stderr)
		);
	};
	let exported_artifacts = || {
		let mut artifacts = std::fs::read_dir(fixture.path())
			.expect("inspect existing-resume fixture")
			.collect::<Result<Vec<_>, _>>()
			.expect("read existing-resume fixture entries")
			.into_iter()
			.map(|entry| entry.path())
			.filter(|path| {
				matches!(
					path.extension().and_then(|extension| extension.to_str()),
					Some("ogdl" | "cubin" | "hsaco")
				)
			})
			.collect::<Vec<_>>();
		artifacts.sort();
		artifacts
	};
	run_source(&initial_source);
	let artifacts_before_resume_only = exported_artifacts();
	run_source(&resume_without_save_source);
	assert_eq!(
		exported_artifacts(),
		artifacts_before_resume_only,
		"resume without a save declaration exported a user-owned artifact"
	);
	for source in [
		&model_only_source,
		&continued_source,
		&missing_fallback_source,
	] {
		run_source(source);
	}

	let mut corrupted_kernel_bytes = std::fs::read(&initial_kernel).expect("read kernel for corruption case");
	let last = corrupted_kernel_bytes
		.last_mut()
		.expect("native kernel is nonempty");
	*last ^= 1;
	let corrupted_kernel = fixture.path().join("corrupted.hsaco");
	std::fs::write(&corrupted_kernel, corrupted_kernel_bytes).expect("write corrupted resume kernel");
	let wrong_backend_kernel = fixture.path().join("wrong-backend.cubin");
	std::fs::copy(&initial_kernel, &wrong_backend_kernel).expect("copy wrong-backend resume kernel");
	let tampered_model = fixture.path().join("wrong-machine.ogdl");
	let mut tampered_model_bytes = std::fs::read(&initial_model).expect("read model for machine-identity case");
	let topology_marker = b"\t\ttopology\t0x";
	let topology_start = tampered_model_bytes
		.windows(topology_marker.len())
		.position(|window| window == topology_marker)
		.map(|position| position + topology_marker.len())
		.expect("saved model contains native topology identity");
	tampered_model_bytes[topology_start] = match tampered_model_bytes[topology_start] {
		b'0' => b'1',
		_ => b'0',
	};
	std::fs::write(&tampered_model, tampered_model_bytes).expect("write wrong-machine semantic model");
	let invalid_cases = [
		(
			"corrupted-kernel.rs",
			training_source(
				Some((&initial_model, Some(&corrupted_kernel))),
				&fixture.path().join("corrupted-output.ogdl"),
				&fixture.path().join("corrupted-output.hsaco"),
			),
			"do not match the digest authenticated by the semantic model",
		),
		(
			"wrong-backend.rs",
			training_source(
				Some((&initial_model, Some(&wrong_backend_kernel))),
				&fixture.path().join("wrong-backend-output.ogdl"),
				&fixture.path().join("wrong-backend-output.hsaco"),
			),
			"has no authenticated .cubin realization",
		),
		(
			"wrong-program.rs",
			training_source(
				Some((&initial_model, Some(&initial_kernel))),
				&fixture.path().join("wrong-program-output.ogdl"),
				&fixture.path().join("wrong-program-output.hsaco"),
			)
			.replace(".epochs(1)", ".epochs(2)"),
			"different static training program",
		),
		(
			"wrong-machine.rs",
			training_source(
				Some((&tampered_model, Some(&initial_kernel))),
				&fixture.path().join("wrong-machine-output.ogdl"),
				&fixture.path().join("wrong-machine-output.hsaco"),
			),
			"different measured topology or discovery profile",
		),
	];
	for (name, source_text, expected_error) in invalid_cases {
		let source = fixture.path().join(name);
		std::fs::write(&source, source_text).expect("write invalid native-resume source");
		let output = Command::new(env!("CARGO_BIN_EXE_recipe"))
			.arg("run")
			.arg(&source)
			.env("XDG_CACHE_HOME", &cache)
			.output()
			.expect("run invalid native-resume source");
		assert!(
			!output.status.success(),
			"invalid native resume unexpectedly succeeded: {name}"
		);
		let stderr = String::from_utf8_lossy(&output.stderr);
		assert!(
			stderr.contains(expected_error),
			"source: {name}\nexpected: {expected_error}\nstderr: {stderr}"
		);
	}

	let initial = decode_checkpoint(
		&std::fs::read(&initial_model).expect("read initial model"),
		CheckpointDecodeLimits::default(),
	)
	.expect("decode initial model");
	let continued = decode_checkpoint(
		&std::fs::read(&continued_model).expect("read continued model"),
		CheckpointDecodeLimits::default(),
	)
	.expect("decode continued model");
	let model_only = decode_checkpoint(
		&std::fs::read(&model_only_model).expect("read model-only continuation"),
		CheckpointDecodeLimits::default(),
	)
	.expect("decode model-only continuation");
	let missing_fallback = decode_checkpoint(
		&std::fs::read(&missing_fallback_model).expect("read missing-kernel continuation"),
		CheckpointDecodeLimits::default(),
	)
	.expect("decode missing-kernel continuation");
	let initial_layer = first_dense_layer(&initial);
	let continued_layer = first_dense_layer(&continued);
	let model_only_layer = first_dense_layer(&model_only);
	let missing_fallback_layer = first_dense_layer(&missing_fallback);
	assert_ne!(
		continued_layer.weight().parameter().bytes(),
		initial_layer.weight().parameter().bytes(),
		"continued training reproduced the deterministic fresh-run weights instead of starting from the saved model"
	);
	assert_ne!(
		continued_layer.weight().first_moment().bytes(),
		initial_layer.weight().first_moment().bytes(),
		"continued training did not advance the admitted AdamW moment state"
	);
	assert_eq!(
		model_only_layer.weight().parameter().bytes(),
		continued_layer.weight().parameter().bytes(),
		"model-only resume and authenticated model-plus-kernel resume advanced different semantic state"
	);
	assert_eq!(
		missing_fallback_layer.weight().parameter().bytes(),
		continued_layer.weight().parameter().bytes(),
		"a missing supplied kernel did not follow the normal recompilation path"
	);
	let initial_kernel_bytes = std::fs::read(&initial_kernel).expect("read initial native kernel");
	assert_eq!(
		std::fs::read(&continued_kernel).expect("read continued native kernel"),
		initial_kernel_bytes,
		"model-plus-kernel resume rebuilt or changed the supplied authenticated native image"
	);
	assert_eq!(
		std::fs::read(&model_only_kernel).expect("read model-only recompiled kernel"),
		initial_kernel_bytes,
		"model-only resume did not deterministically recompile the native image"
	);
	assert_eq!(
		std::fs::read(&missing_fallback_kernel).expect("read missing-path recompiled kernel"),
		initial_kernel_bytes,
		"missing resume kernel did not deterministically recompile the native image"
	);
	let run_directory = cache.join("recipe-next/run");
	let leftovers = std::fs::read_dir(&run_directory)
		.expect("inspect private native source-runner directory")
		.collect::<Result<Vec<_>, _>>()
		.expect("read private native source-runner entries");
	assert!(
		leftovers.is_empty(),
		"compiled native source-runner artifacts remained in {}: {leftovers:?}",
		run_directory.display()
	);
}
