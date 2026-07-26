use std::num::NonZeroU64;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use recipe::{ValidationMetricFamily, ValidationMetricStatus, ValidationUnavailableReason};

#[path = "../src/validation_reporting.rs"]
mod validation_reporting;

static FIXTURE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);

impl Fixture {
	fn csv(bytes: &[u8]) -> Self {
		let sequence = FIXTURE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
		let path = std::env::temp_dir().join(format!(
			"recipe-training-reporting-{}-{sequence}.csv",
			std::process::id()
		));
		std::fs::write(&path, bytes).expect("write training-reporting fixture");
		Self(path)
	}

	fn path(&self) -> &Path {
		&self.0
	}
}

impl Drop for Fixture {
	fn drop(&mut self) {
		let _ = std::fs::remove_file(&self.0);
	}
}

#[test]
fn zero_known_validation_status_has_exact_user_facing_text() {
	let binary = ValidationMetricStatus::Unavailable {
		family: ValidationMetricFamily::Binary,
		reason: ValidationUnavailableReason::NoKnownTargets,
		split_rows: 2,
	};
	assert_eq!(
		validation_reporting::unavailable_validation_line(binary).as_deref(),
		Some("validation unavailable  family binary  reason no-known-targets  known_rows 0  split_rows 2")
	);
	let multiclass = ValidationMetricStatus::Unavailable {
		family: ValidationMetricFamily::Multiclass,
		reason: ValidationUnavailableReason::NoKnownTargets,
		split_rows: 11,
	};
	assert_eq!(
		validation_reporting::unavailable_validation_line(multiclass).as_deref(),
		Some("validation unavailable  family multiclass  reason no-known-targets  known_rows 0  split_rows 11")
	);
	let single_class = ValidationMetricStatus::Unavailable {
		family: ValidationMetricFamily::Binary,
		reason: ValidationUnavailableReason::SingleKnownClass {
			known_rows: NonZeroU64::new(7).unwrap(),
		},
		split_rows: 9,
	};
	assert_eq!(
		validation_reporting::unavailable_validation_line(single_class).as_deref(),
		Some("validation unavailable  family binary  reason single-known-class  known_rows 7  split_rows 9")
	);
	assert_eq!(
		validation_reporting::unavailable_validation_line(ValidationMetricStatus::Available {
			family: ValidationMetricFamily::Binary,
			known_rows: NonZeroU64::MIN,
		}),
		None
	);
}

#[test]
fn public_compile_and_report_boundaries_preserve_typed_validation_status() {
	let fixture = Fixture::csv(b"feature,target\n1,0\n2,1\n3,\n4,\n");
	let data = recipe::recipe
		.data(fixture.path().to_str().expect("temporary path is UTF-8"))
		.target("target")
		.norm(recipe::z_score)
		.split(0.5);
	let model = recipe::recipe.model().layer(1).loss(recipe::bce);
	let train = recipe::recipe
		.train()
		.optimizer(recipe::adamw)
		.batch(0.5)
		.epochs(1)
		.lr(0.001)
		.cos()
		.early_stop(recipe::AuPrc, 1)
		.log([recipe::AuPrc, recipe::CalibrationError]);

	let compiled = recipe::compile_training(&train, &data, &model).expect("compile zero-known validation training");
	assert_eq!(
		compiled.outputs().validation_status,
		ValidationMetricStatus::Unavailable {
			family: ValidationMetricFamily::Binary,
			reason: ValidationUnavailableReason::NoKnownTargets,
			split_rows: 2,
		}
	);
	let accessor: fn(&recipe::TrainingReport) -> ValidationMetricStatus = recipe::TrainingReport::validation_status;
	let _ = accessor;
}
