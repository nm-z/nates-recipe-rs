use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static FIXTURE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct FixtureCrate {
	root: PathBuf,
	target: PathBuf,
}

impl FixtureCrate {
	fn new(recipe_root: &Path) -> Self {
		let sequence = FIXTURE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
		let root = std::env::temp_dir().join(format!(
			"recipe-public-api-{}-{sequence}",
			std::process::id()
		));
		let source = root.join("src");
		std::fs::create_dir_all(&source).expect("create public API fixture crate");
		let dependency = recipe_root
			.display()
			.to_string()
			.replace('\\', "\\\\")
			.replace('"', "\\\"");
		std::fs::write(
			root.join("Cargo.toml"),
			format!(
				concat!(
					"[package]\n",
					"name = \"recipe-public-api-probe\"\n",
					"version = \"0.0.0\"\n",
					"edition = \"2024\"\n\n",
					"[workspace]\n\n",
					"[dependencies]\n",
					"recipe = {{ path = \"{dependency}\" }}\n",
				),
				dependency = dependency,
			),
		)
		.expect("write public API fixture manifest");
		Self {
			root,
			target: recipe_root.join("target/public-api-compile"),
		}
	}

	fn check(&self, source: &str) -> Output {
		std::fs::write(self.root.join("src/lib.rs"), source).expect("write public API fixture source");
		let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
		Command::new(cargo)
			.arg("check")
			.arg("--offline")
			.arg("--quiet")
			.arg("--manifest-path")
			.arg(self.root.join("Cargo.toml"))
			.env("CARGO_TARGET_DIR", &self.target)
			.env("CARGO_TERM_COLOR", "never")
			.env_remove("RUSTFLAGS")
			.env_remove("CARGO_ENCODED_RUSTFLAGS")
			.output()
			.expect("run cargo check for public API fixture")
	}
}

impl Drop for FixtureCrate {
	fn drop(&mut self) {
		let _ = std::fs::remove_dir_all(&self.root);
	}
}

fn fixture(path: &str) -> String {
	std::fs::read_to_string(
		Path::new(env!("CARGO_MANIFEST_DIR"))
			.join("tests/public_api")
			.join(path),
	)
	.unwrap_or_else(|error| panic!("read public API fixture {path}: {error}"))
}

#[test]
fn public_surface_keeps_canonical_calls_and_rejects_banned_calls() {
	let root = Path::new(env!("CARGO_MANIFEST_DIR"));
	let crate_under_test = FixtureCrate::new(root);

	let pass = crate_under_test.check(&fixture("pass/canonical.rs"));
	assert!(
		pass.status.success(),
		"canonical public API did not compile:\n{}",
		String::from_utf8_lossy(&pass.stderr)
	);

	for (path, diagnostic) in [
		("fail/model_lr.rs", "no method named `lr`"),
		("fail/model_optimizer.rs", "no method named `optimizer`"),
		(
			"fail/into_data_sources.rs",
			"trait `IntoDataSources` is private",
		),
		(
			"fail/into_residual_branch.rs",
			"trait `IntoResidualBranch` is private",
		),
		(
			"fail/convolution_stride.rs",
			"variant `LayerSpec::Convolution` has no field named `stride`",
		),
		(
			"fail/condition_new.rs",
			"associated function `new` is private",
		),
		(
			"fail/recall_at.rs",
			"cannot find function, tuple struct or tuple variant `RecallAt` in crate `recipe`",
		),
		("fail/cosine_decay.rs", "no method named `cosine_decay`"),
		("fail/data_validate.rs", "method `validate` is private"),
		("fail/model_validate.rs", "method `validate` is private"),
		("fail/train_validate.rs", "method `validate` is private"),
		("fail/infer_validate.rs", "method `validate` is private"),
		("fail/train_declare.rs", "no method named `declare`"),
		("fail/infer_declare.rs", "no method named `declare`"),
		("fail/net.rs", "no method named `net`"),
		("fail/try_run.rs", "method `try_run` is private"),
		("fail/run_with.rs", "no method named `run_with`"),
		("fail/try_run_with.rs", "method `try_run_with` is private"),
		("fail/try_save.rs", "no method named `try_save`"),
		("fail/train_batch.rs", "no method named `batch`"),
		(
			"fail/report_checkpoint.rs",
			"no method named `checkpoint_manifest`",
		),
		(
			"fail/report_execution.rs",
			"no method named `into_execution`",
		),
		(
			"fail/save_unit.rs",
			"the trait `From<()>` is not implemented",
		),
		("fail/calibrate.rs", "no method named `calibrate`"),
		("fail/linear.rs", "no method named `linear`"),
		(
			"fail/load_with_input_width.rs",
			"this method takes 1 argument but 2 arguments were supplied",
		),
		(
			"fail/knn_outputs.rs",
			"this method takes 1 argument but 2 arguments were supplied",
		),
		("fail/model_input_width.rs", "no method named `input_width`"),
		(
			"fail/evaluate_with_explicit_declarations.rs",
			"this method takes 0 arguments but 2 arguments were supplied",
		),
	] {
		let output = crate_under_test.check(&fixture(path));
		let stderr = String::from_utf8_lossy(&output.stderr);
		assert!(
			!output.status.success(),
			"banned public API fixture {path} unexpectedly compiled"
		);
		assert!(
			stderr.contains(diagnostic),
			"banned public API fixture {path} failed for the wrong reason; expected {diagnostic:?}:\n{stderr}"
		);
	}
}
