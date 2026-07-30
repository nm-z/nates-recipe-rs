use std::collections::BTreeMap;
use std::num::NonZeroU64;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static FIXTURE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);

impl Fixture {
	fn write(extension: &str, bytes: &[u8]) -> Self {
		let sequence = FIXTURE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
		let path = std::env::temp_dir().join(format!(
			"recipe-data-bridge-{}-{sequence}.{extension}",
			std::process::id()
		));
		std::fs::write(&path, bytes).expect("write data-bridge fixture");
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

struct FixtureDirectory(PathBuf);

impl FixtureDirectory {
	fn new() -> Self {
		let sequence = FIXTURE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
		let path = std::env::temp_dir().join(format!(
			"recipe-data-directory-{}-{sequence}",
			std::process::id()
		));
		std::fs::create_dir(&path).expect("create data-directory fixture");
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

fn declaration_error(error: recipe::InferenceError) -> recipe::DeclarationError {
	match error {
		recipe::InferenceError::Declaration(error) => error,
		other => panic!("expected inference declaration error, got {other}"),
	}
}

#[test]
fn facade_reaches_every_entry_point_without_external_work() {
	let data = recipe::recipe.data("not-opened-during-construction.csv");
	assert_eq!(data.source(), "not-opened-during-construction.csv");
	assert_eq!(
		recipe::recipe
			.data(["a.csv", "b.csv"])
			.set("c.zip")
			.sources(),
		["a.csv", "b.csv", "c.zip"]
	);
	assert_eq!(
		recipe::recipe
			.data(vec![String::from("a.csv"), String::from("b.csv")])
			.sources(),
		["a.csv", "b.csv"]
	);
	assert!(recipe::recipe.model().layers().is_empty());
	assert_eq!(recipe::recipe.train().epoch_bound(), None);
	assert!(recipe::recipe.infer().log_items().is_empty());
	let empty = recipe::recipe.data(());
	assert!(empty.sources().is_empty());
	assert_eq!(empty.set("later.csv").sources(), ["later.csv"]);
}

#[test]
fn data_bridge_auto_loads_filters_and_exactly_splits_csv_and_tsv() {
	use recipe::engine::ingest::{SemanticType, VectorRole};

	let csv = Fixture::write(
		"csv",
		b"id,Age,note,No-show\n\
		1,-1,\"first sentence with several words\",No\n\
		2,20,\"second sentence with several words\",Yes\n\
		3,30,\"third sentence with several words\",No\n\
		4,40,\"fourth sentence with several words\",Yes\n\
		5,50,\"fifth sentence with several words\",No\n",
	);
	let declaration = recipe::recipe
		.data(csv.path().to_str().unwrap())
		.target("No-show")
		.exclude("id")
		.exclude(recipe::cond!(Age < 0))
		.split(0.75);
	let prepared = recipe::prepare_data(&declaration).unwrap();
	assert_eq!(prepared.source_row_count(), 5);
	assert_eq!(prepared.retained_source_rows(), [1, 2, 3, 4]);
	assert_eq!(prepared.excluded_source_rows(), [0]);
	assert_eq!(prepared.train().source_rows(), [1, 2, 3]);
	assert_eq!(prepared.validation().source_rows(), [4]);
	assert_eq!(
		prepared
			.vectors()
			.iter()
			.map(|vector| (vector.name(), vector.role(), vector.semantic_type()))
			.collect::<Vec<_>>(),
		[
			(
				b"Age".as_slice(),
				VectorRole::Feature,
				SemanticType::Numeric
			),
			(b"note".as_slice(), VectorRole::Feature, SemanticType::Text),
			(
				b"No-show".as_slice(),
				VectorRole::Target,
				SemanticType::Categorical
			),
		]
	);

	let tsv = Fixture::write("tsv", b"feature\tlabel\n1\tyes\n2\tno\n3\tyes\n4\tno\n");
	let prepared = recipe::prepare_data(
		&recipe::recipe
			.data(tsv.path().to_str().unwrap())
			.target("label")
			.split(0.5),
	)
	.unwrap();
	assert_eq!(prepared.train().len(), 2);
	assert_eq!(prepared.validation().len(), 2);
}

#[test]
fn data_bridge_distills_recursive_image_folders_and_modeled_metadata() {
	use recipe::engine::ingest::{SemanticType, VectorRole};

	let png = b"\x89PNG\r\n\x1a\n\x00\x00\x00\x0dIHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x06\x00\x00\x00\x1f\x15\xc4\x89";
	let directory = FixtureDirectory::new();
	for (folder, files) in [
		("cats", ["000001.png", "000002.png"]),
		("dogs", ["hash-a.png", "hash-b.png"]),
	] {
		let path = directory.path().join(folder);
		std::fs::create_dir(&path).unwrap();
		for file in files {
			std::fs::write(path.join(file), png).unwrap();
		}
	}
	let prepared = recipe::prepare_data(
		&recipe::recipe
			.data(directory.path().to_str().unwrap())
			.target("folder")
			.split(0.5),
	)
	.unwrap();

	assert_eq!(prepared.source_row_count(), 4);
	assert_eq!(prepared.train().len(), 2);
	assert_eq!(prepared.validation().len(), 2);
	assert!(prepared.vectors().iter().any(|vector| {
		vector.name() == b"folder"
			&& vector.role() == VectorRole::Target
			&& vector.semantic_type() == SemanticType::Categorical
	}));
	assert!(prepared.vectors().iter().any(|vector| {
		vector.name() == b"image"
			&& vector.role() == VectorRole::Feature
			&& vector.semantic_type() == SemanticType::Image
	}));
}

#[test]
fn data_bridge_appends_declared_sources_in_order() {
	use recipe::engine::ingest::{PreparedValues, SemanticType};

	let first = Fixture::write("csv", b"value,label\n10,yes\n11,no\n");
	let second = Fixture::write("csv", b"value,label\n20,no\n21,yes\n");
	let declaration = recipe::recipe
		.data(first.path().to_str().unwrap())
		.set(second.path().to_str().unwrap())
		.target("label")
		.split(0.5);
	assert_eq!(
		declaration.sources(),
		[
			first.path().to_str().unwrap(),
			second.path().to_str().unwrap()
		]
	);
	let prepared = recipe::prepare_data(&declaration).unwrap();
	assert_eq!(prepared.source_row_count(), 4);
	assert_eq!(prepared.retained_source_rows(), [0, 1, 2, 3]);
	assert_eq!(prepared.train().source_rows(), [0, 1]);
	assert_eq!(prepared.validation().source_rows(), [2, 3]);
	let source_index = prepared
		.vectors()
		.iter()
		.find(|vector| vector.name() == b"source_index")
		.unwrap();
	assert_eq!(source_index.semantic_type(), SemanticType::Categorical);
	assert_eq!(
		source_index.values(),
		&PreparedValues::I32(vec![Some(0), Some(0), Some(1), Some(1)])
	);
}

#[test]
fn data_bridge_prepares_multiple_targets_in_declared_order_without_merging_their_semantics() {
	use recipe::engine::ingest::{DenseMatrix, PartitionKind, SemanticType, VectorRole};

	let csv = Fixture::write(
		"csv",
		b"feature,target_count,target_score,ignored\n\
		1,10,1.5,a\n\
		2,20,2.5,b\n\
		3,30,3.5,c\n\
		4,40,4.5,d\n",
	);
	let declaration = recipe::recipe
		.data(csv.path().to_str().unwrap())
		.target(["target_score", "target_count"])
		.exclude("ignored")
		.split(0.5);
	let prepared = recipe::prepare_data(&declaration).unwrap();
	let targets = prepared
		.target_source_indices()
		.iter()
		.map(|source_index| {
			prepared
				.vectors()
				.iter()
				.find(|vector| vector.source_index() == *source_index && vector.role() == VectorRole::Target)
				.unwrap()
		})
		.map(|vector| (vector.name(), vector.semantic_type()))
		.collect::<Vec<_>>();
	assert_eq!(
		targets,
		[
			(b"target_score".as_slice(), SemanticType::Numeric),
			(b"target_count".as_slice(), SemanticType::Numeric),
		]
	);
	assert_eq!(
		prepared
			.fixed_dense_matrix(VectorRole::Target, PartitionKind::Train)
			.unwrap(),
		DenseMatrix::F32Bits {
			rows: 2,
			columns: 2,
			values: vec![
				10.0f32.to_bits(),
				1.5f32.to_bits(),
				20.0f32.to_bits(),
				2.5f32.to_bits()
			],
		}
	);
}

#[test]
fn public_dense_training_uses_all_declared_targets_in_their_declared_order() {
	let csv = Fixture::write(
		"csv",
		b"feature,winner_model_a,winner_model_b,winner_tie\n\
		1,1,0,0\n\
		2,0,1,0\n\
		3,0,0,1\n\
		4,1,0,0\n\
		5,0,1,0\n\
		6,0,0,1\n\
		7,1,0,0\n",
	);
	let data = recipe::recipe
		.data(csv.path().to_str().unwrap())
		.target(["winner_model_b", "winner_model_a", "winner_tie"])
		.norm(recipe::z_score)
		.split(5.0 / 7.0);
	let policy = recipe::recipe
		.train()
		.optimizer(recipe::adamw)
		.epochs(1)
		.lr(0.0001)
		.cos();
	for (loss, expected_task) in [
		(
			recipe::bce,
			recipe_training::DenseTask::MultiTargetBinaryClassification {
				first_target_vector: 2,
				target_count: 3,
			},
		),
		(
			recipe::ce,
			recipe_training::DenseTask::JointMulticlassClassification {
				first_target_vector: 2,
				target_count: 3,
			},
		),
		(
			recipe::mse,
			recipe_training::DenseTask::MultiTargetRegression {
				first_target_vector: 2,
				target_count: 3,
			},
		),
	] {
		let model = recipe::recipe.model().layer(3).loss(loss);
		let compiled = recipe::compile_training(&policy, &data, &model).unwrap();
		assert_eq!(compiled.dataset_schema().targets(), [2, 1, 3]);
		assert_eq!(compiled.dataset_schema().task(), expected_task);
		assert_eq!(compiled.dataset_schema().output_width(), 3);
		compiled.graph().validate().unwrap();
		compiled.program().validate().unwrap();
		let checkpoint = recipe_training::CheckpointManifest::from_compiled(&compiled).unwrap();
		assert_eq!(checkpoint.format_version(), 11);
		assert_eq!(checkpoint.target_source_indices(), [2, 1, 3]);
		assert_eq!(checkpoint.task(), expected_task);
	}
}

#[test]
fn public_data_distillation_opens_a_checked_in_zip_through_the_same_bounded_source_boundary() {
	let archive = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/datasets/uci-sms-tab/smsspamcollection.zip");
	let distilled = recipe::distill_data(&recipe::recipe.data(archive.to_str().unwrap())).unwrap();
	assert!(distilled.sample_count() > 0);
	assert!(distilled.vector_count() > 0);
	assert_eq!(distilled.file_count(), 2);
}

#[test]
fn data_bridge_rejects_incomplete_declarations_before_io() {
	let no_sources = recipe::recipe
		.data([] as [&str; 0])
		.target("label")
		.split(0.5);
	assert!(matches!(
		recipe::prepare_data(&no_sources),
		Err(recipe::DataPreparationError::Declaration(_))
	));

	let no_target = recipe::recipe.data("never-opened.csv").split(0.8);
	assert!(matches!(
		recipe::prepare_data(&no_target),
		Err(recipe::DataPreparationError::MissingTargets)
	));

	let no_split = recipe::recipe.data("never-opened.csv").target("label");
	assert!(matches!(
		recipe::prepare_data(&no_split),
		Err(recipe::DataPreparationError::MissingSplit)
	));
}

#[test]
fn data_bridge_propagates_caller_ingest_bounds() {
	use recipe::engine::ingest::{DatasetSourceErrorKind, IngestLimits};

	let fixture = Fixture::write("csv", b"x,label\n1,yes\n2,no\n");
	let declaration = recipe::recipe
		.data(fixture.path().to_str().unwrap())
		.target("label")
		.split(0.5);
	let error = recipe::prepare_data_with_limits(&declaration, IngestLimits::new(4, 8, 4, 16).unwrap()).unwrap_err();
	assert!(matches!(
		error,
		recipe::DataPreparationError::Source(ref source)
			if source.kind == DatasetSourceErrorKind::LimitExceeded
	));
}

#[test]
fn native_profile_loader_requires_an_explicit_identity_filename() {
	assert!(matches!(
		recipe::load_cached_measured_profile("/tmp/current.recipe-profile"),
		Err(recipe::NativePreparationError::InvalidCachePath { .. })
	));
	assert!(matches!(
		recipe::load_cached_measured_profile("/tmp/measured-v5-ABCDEF.recipe-profile"),
		Err(recipe::NativePreparationError::InvalidCachePath { .. })
	));
}

#[test]
#[ignore = "loads the explicitly selected private measured-profile cache"]
fn native_profile_loader_reads_the_exact_selected_cache() {
	let path = std::env::var_os("RECIPE_TEST_PROFILE").expect("RECIPE_TEST_PROFILE must name the exact cache");
	let profile = recipe::load_cached_measured_profile(path).unwrap();
	assert!(!profile.topology.devices.is_empty());
	assert_eq!(profile.discovery.topology, profile.topology.identity);
}

#[test]
#[ignore = "reopens the exact current bare-metal CUDA/HSA devices"]
fn current_native_preparation_reopens_compiler_and_host_plans() {
	recipe::with_current_native_preparation(|profile, _config, scope| {
		assert!(!scope.targets().devices().is_empty());
		assert_eq!(scope.targets().machine(), scope.bindings().machine());
		assert_eq!(scope.host().machine(), scope.bindings().machine());
		assert_eq!(
			scope.targets().devices().len(),
			scope.bindings().cuda().len() + scope.bindings().hsa().len()
		);
		assert_eq!(
			scope.targets().devices().len(),
			profile
				.discovery
				.devices
				.iter()
				.filter(|device| device.calculation.is_some())
				.count()
		);
		let _compiler = scope
			.targets()
			.deferred_compiler()
			.map_err(recipe::NativePreparationError::TargetSpecification)?;
		let _host = scope
			.host()
			.backend_config(recipe::engine::core::RunId::new(1), 1, 4096)
			.map_err(|error| recipe::NativePreparationError::LocalConfiguration(error.to_string()))?;
		Ok(())
	})
	.unwrap();
}

#[test]
#[ignore = "loads the checked-in 11 MiB integration dataset"]
fn data_bridge_prepares_the_no_show_dataset_without_derived_features() {
	let path =
		Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/datasets/no-show-appointments/KaggleV2-May-2016.csv");
	let declaration = recipe::recipe
		.data(path.to_str().unwrap())
		.target("No-show")
		.exclude(["PatientId", "AppointmentID"])
		.exclude(recipe::cond!(Age < 0))
		.split(0.8);
	let prepared = recipe::prepare_data(&declaration).unwrap();
	assert_eq!(prepared.source_row_count(), 110_527);
	assert_eq!(prepared.retained_source_rows().len(), 110_526);
	assert_eq!(prepared.excluded_source_rows().len(), 1);
	assert_eq!(prepared.vectors().len(), 12);
	assert_eq!(prepared.train().len(), 88_420);
	assert_eq!(prepared.validation().len(), 22_106);
}

#[test]
#[ignore = "loads and compiles the checked-in 11 MiB integration dataset"]
fn public_no_show_training_declaration_compiles_to_the_static_program() {
	let path =
		Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/datasets/no-show-appointments/KaggleV2-May-2016.csv");
	let data = recipe::recipe
		.data(path.to_str().unwrap())
		.target("No-show")
		.exclude(["AppointmentID", "PatientId"])
		.exclude(recipe::cond!(Age < 0))
		.norm(recipe::z_score)
		.split(0.8);
	let model = recipe::recipe
		.model()
		.layer(128)
		.silu()
		.layer(128)
		.silu()
		.layer(1)
		.loss(recipe::bce)
		.grad(recipe::clip(1.0));
	let policy = recipe::recipe
		.train()
		.optimizer(recipe::adamw)
		.epochs(100)
		.lr(0.0001)
		.warmup(5)
		.cos()
		.log([
			recipe::Loss,
			recipe::AuRoc,
			recipe::AuPrc,
			recipe::Brier,
			recipe::CalibrationError,
		]);

	let compiled = recipe::compile_training(&policy, &data, &model).unwrap();
	assert_eq!(compiled.bounds().train_rows, 88_420);
	assert_eq!(
		compiled
			.bounds()
			.training_iterations
			.finite()
			.unwrap()
			.get(),
		100
	);
	assert_eq!(compiled.bounds().calibration_iterations, 0);
	assert_eq!(compiled.bounds().iterations.finite().unwrap().get(), 100);
	compiled.graph().validate().unwrap();
	compiled.program().validate().unwrap();
	assert!(!compiled.program().to_ogdl().unwrap().is_empty());
}

#[test]
fn public_numeric_regression_target_compiles_without_binary_reinterpretation() {
	let csv = Fixture::write(
		"csv",
		b"feature_one,feature_two,SalePrice\n\
		1,10,208500\n\
		2,20,181500\n\
		3,30,223500\n\
		4,40,140000\n\
		5,50,250000\n\
		6,60,307000\n\
		7,70,129900\n",
	);
	let data = recipe::recipe
		.data(csv.path().to_str().unwrap())
		.target("SalePrice")
		.norm(recipe::z_score)
		.split(5.0 / 7.0);
	let model = recipe::recipe
		.model()
		.layer(8)
		.silu()
		.layer(1)
		.loss(recipe::mse);
	let policy = recipe::recipe
		.train()
		.optimizer(recipe::adamw)
		.epochs(1)
		.lr(0.0001)
		.cos();

	let compiled = recipe::compile_training(&policy, &data, &model).unwrap();
	assert_eq!(
		compiled.dataset_schema().task(),
		recipe_training::DenseTask::ScalarRegression { target_vector: 2 }
	);
	let target = compiled
		.dataset_schema()
		.vectors()
		.iter()
		.find(|vector| vector.role() == recipe::engine::ingest::VectorRole::Target)
		.unwrap();
	assert_eq!(
		target.semantic_type(),
		recipe::engine::ingest::SemanticType::Numeric
	);
	assert_eq!(
		target.encoding(),
		recipe::engine::ingest::VectorEncoding::I32
	);
	compiled.graph().validate().unwrap();
	compiled.program().validate().unwrap();
}

#[test]
fn missing_resume_model_starts_fresh_training() {
	let directory = FixtureDirectory::new();
	let missing_model = directory.path().join("not-created.ogdl");
	let csv = Fixture::write(
		"csv",
		b"feature,target\n\
		1,10\n\
		2,20\n\
		3,30\n\
		4,40\n",
	);
	let data = recipe::recipe
		.data(csv.path().to_str().unwrap())
		.target("target")
		.norm(recipe::z_score)
		.split(0.75);
	let model = recipe::recipe.model().layer(1).loss(recipe::mse);
	let policy = recipe::recipe
		.train()
		.optimizer(recipe::adamw)
		.epochs(1)
		.lr(0.0001)
		.cos()
		.resume(missing_model.to_str().unwrap());

	let compiled = recipe::compile_training(&policy, &data, &model).unwrap();
	assert_eq!(compiled.bounds().train_rows, 3);
}

#[test]
fn artifact_declarations_route_exact_forms_and_reject_ambiguous_or_kernel_only_resume() {
	let directory = FixtureDirectory::new();
	let csv = directory.path().join("train.csv");
	std::fs::write(&csv, "feature,target\n1,10\n2,20\n3,30\n4,40\n").unwrap();
	let data = recipe::recipe
		.data(csv.to_str().unwrap())
		.target("target")
		.norm(recipe::z_score)
		.split(0.75);
	let model = recipe::recipe.model().layer(1).loss(recipe::mse);
	let base = recipe::recipe
		.train()
		.optimizer(recipe::adamw)
		.epochs(1)
		.lr(0.0001)
		.cos();
	let missing_model = directory.path().join("missing.ogdl");
	let missing_kernel = directory.path().join("missing.hsaco");
	for policy in [
		base.clone().save("model.ogdl"),
		base.clone().save("kernel.hsaco"),
		base.clone()
			.__recipe_save_pair("model.ogdl", "kernel.hsaco"),
		base.clone().resume(missing_model.to_str().unwrap()),
		base.clone().__recipe_resume_pair(
			missing_model.to_str().unwrap(),
			missing_kernel.to_str().unwrap(),
		),
	] {
		recipe::compile_training(&policy, &data, &model).expect("valid artifact declaration compiles");
	}
	for policy in [
		base.clone().resume("kernel.hsaco"),
		base.clone().save("model.bin"),
		base.clone().save("one.ogdl").save("two.hsaco"),
		base.clone().resume("one.ogdl").resume("two.ogdl"),
		base.clone()
			.__recipe_save_pair("kernel.hsaco", "model.ogdl"),
		base.__recipe_resume_pair("kernel.hsaco", "model.ogdl"),
	] {
		let error = recipe::compile_training(&policy, &data, &model).unwrap_err();
		assert!(
			matches!(error, recipe::TrainingError::Declaration(_)),
			"{error}"
		);
	}
}

#[test]
fn public_multiclass_compile_and_checkpoint_preserve_reserved_class_and_declared_output() {
	let csv = Fixture::write(
		"csv",
		b"feature_one,feature_two,target\n\
		1,10,red\n\
		2,20,blue\n\
		3,30,green\n\
		4,40,red\n\
		5,50,blue\n\
		6,60,green\n\
		7,70,red\n\
		8,80,blue\n\
		9,90,yellow\n\
		10,100,green\n",
	);
	let data = recipe::recipe
		.data(csv.path().to_str().unwrap())
		.target("target")
		.norm(recipe::z_score)
		.split(0.8);
	let model = recipe::recipe
		.model()
		.layer(4)
		.silu()
		.layer(4)
		.loss(recipe::ce);
	let policy = recipe::recipe
		.train()
		.optimizer(recipe::adamw)
		.epochs(1)
		.lr(0.0001)
		.cos();

	let compiled = recipe::compile_training(&policy, &data, &model).unwrap();
	assert_eq!(
		compiled.dataset_schema().task(),
		recipe_training::DenseTask::MulticlassClassification {
			target_vector: 2,
			class_count: 4,
			reserved_code: 3,
		}
	);
	assert_eq!(compiled.output_adapter(), None);
	let checkpoint = recipe_training::CheckpointManifest::from_compiled(&compiled).unwrap();
	assert_eq!(checkpoint.task(), compiled.dataset_schema().task());
	assert_eq!(checkpoint.output_adapter(), None);
	assert_eq!(
		checkpoint.vectors()[2].metadata(),
		compiled.dataset_schema().vectors()[2].metadata()
	);
	compiled.graph().validate().unwrap();
	compiled.program().validate().unwrap();
}

#[test]
fn train_lr_defaults_to_linear_decay_and_explicit_schedules_override_it() {
	assert_eq!(
		recipe::recipe.train().lr(0.001).learning_rate_schedule(),
		Some(recipe::LearningRateSchedule::LinearDecay)
	);
	assert_eq!(
		recipe::recipe
			.train()
			.lr(0.001)
			.cos()
			.learning_rate_schedule(),
		Some(recipe::LearningRateSchedule::CosineDecay)
	);
	assert_eq!(
		recipe::recipe
			.train()
			.lr(0.001)
			.exp()
			.learning_rate_schedule(),
		Some(recipe::LearningRateSchedule::ExponentialDecay)
	);
	assert_eq!(
		recipe::recipe
			.train()
			.lr(0.001)
			.cos()
			.exp()
			.learning_rate_schedule(),
		Some(recipe::LearningRateSchedule::ExponentialDecay)
	);
	assert_eq!(
		recipe::recipe
			.train()
			.lr(0.001)
			.exp()
			.cos()
			.learning_rate_schedule(),
		Some(recipe::LearningRateSchedule::CosineDecay)
	);
}

#[test]
fn facade_builders_are_immutable_validated_declarations() {
	let base_data = recipe::recipe.data("not-opened.csv");
	let prepared_data = base_data
		.clone()
		.set("also-not-opened.csv")
		.target(["label", "weight"])
		.exclude("ignored_*")
		.norm(recipe::min_max)
		.split(0.8);
	assert_eq!(base_data.sources(), ["not-opened.csv"]);
	assert_eq!(
		prepared_data.sources(),
		["not-opened.csv", "also-not-opened.csv"]
	);
	assert_eq!(prepared_data.targets(), ["label", "weight"]);
	assert_eq!(prepared_data.exclusions(), ["ignored_*"]);
	assert_eq!(prepared_data.split_fraction(), Some(0.8_f32));
	assert_eq!(
		prepared_data.normalization(),
		Some(recipe::DataNormalization::MinMax)
	);

	let train = recipe::recipe
		.train()
		.epochs(3)
		.lr(0.001)
		.cos()
		.log([recipe::Loss, recipe::Accuracy, recipe::Device])
		.plot([recipe::Loss, recipe::Accuracy]);
	assert_eq!(train.epoch_bound(), Some(3));
	assert_eq!(train.learning_rate(), Some(0.001));
	assert_eq!(
		recipe::recipe.train().log(recipe::Loss).log_items(),
		[recipe::Loss]
	);
	assert_eq!(
		recipe::recipe
			.train()
			.lr(0.001)
			.cos()
			.learning_rate_schedule(),
		Some(recipe::LearningRateSchedule::CosineDecay)
	);
	assert_eq!(
		recipe::recipe
			.train()
			.lr(0.001)
			.exp()
			.learning_rate_schedule(),
		Some(recipe::LearningRateSchedule::ExponentialDecay)
	);
	assert_eq!(
		recipe::recipe
			.model()
			.layer(8)
			.cos()
			.layer(8)
			.exp()
			.layer(8)
			.log()
			.layer(8)
			.ln()
			.layer(8)
			.huber()
			.layer(8)
			.tan()
			.layer(8)
			.relu()
			.layer(8)
			.gelu()
			.layers(),
		[
			recipe::LayerSpec::Dense {
				units: 8,
				operations: vec![recipe::LayerOperation::Activation(
					recipe::Activation::Cosine
				)],
			},
			recipe::LayerSpec::Dense {
				units: 8,
				operations: vec![recipe::LayerOperation::Activation(
					recipe::Activation::Exponential
				)],
			},
			recipe::LayerSpec::Dense {
				units: 8,
				operations: vec![recipe::LayerOperation::Activation(
					recipe::Activation::Logarithm
				)],
			},
			recipe::LayerSpec::Dense {
				units: 8,
				operations: vec![recipe::LayerOperation::Activation(
					recipe::Activation::NaturalLogarithm
				)],
			},
			recipe::LayerSpec::Dense {
				units: 8,
				operations: vec![recipe::LayerOperation::Activation(
					recipe::Activation::Huber
				)],
			},
			recipe::LayerSpec::Dense {
				units: 8,
				operations: vec![recipe::LayerOperation::Activation(
					recipe::Activation::Tangent
				)],
			},
			recipe::LayerSpec::Dense {
				units: 8,
				operations: vec![recipe::LayerOperation::Activation(recipe::Activation::Relu)],
			},
			recipe::LayerSpec::Dense {
				units: 8,
				operations: vec![recipe::LayerOperation::Activation(recipe::Activation::Gelu)],
			},
		]
	);
	assert_eq!(
		recipe::recipe
			.model()
			.layer(32)
			.norm(recipe::layer_norm)
			.silu()
			.layer(32)
			.silu()
			.norm(recipe::layer_norm)
			.layers(),
		[
			recipe::LayerSpec::Dense {
				units: 32,
				operations: vec![
					recipe::LayerOperation::Normalization(recipe::LayerNormalization::LayerNorm),
					recipe::LayerOperation::Activation(recipe::Activation::Silu),
				],
			},
			recipe::LayerSpec::Dense {
				units: 32,
				operations: vec![
					recipe::LayerOperation::Activation(recipe::Activation::Silu),
					recipe::LayerOperation::Normalization(recipe::LayerNormalization::LayerNorm),
				],
			},
		]
	);

	let declare_inference_sequence = || {
		let data = recipe::recipe
			.data("not-opened.csv")
			.set("also-not-opened.csv")
			.target(["label", "weight"])
			.exclude("ignored_*")
			.norm(recipe::min_max)
			.split(0.8);
		let model = recipe::recipe
			.model()
			.layer(32)
			.relu()
			.embed(8)
			.vocab(1024)
			.attn(4)
			.loss(recipe::mse);
		(data, model)
	};
	let (_inference_data, _inference_model) = declare_inference_sequence();
	let inference = recipe::recipe.infer().log([recipe::Time, recipe::Device]);
	assert_eq!(inference.log_items(), [recipe::Time, recipe::Device]);
	let (_inference_data, _inference_model) = declare_inference_sequence();
	let target_derived = recipe::recipe.infer().log(recipe::Loss);
	assert_eq!(target_derived.log_items(), [recipe::Loss]);
	let error = declaration_error(
		target_derived
			.evaluate()
			.expect_err("target-free inference accepted a target-derived metric"),
	);
	assert_eq!(
		error.kind,
		recipe::DeclarationErrorKind::InvalidInferenceConfiguration,
	);
	assert!(error.detail.starts_with("Loss:"));
	let (_inference_data, _inference_model) = declare_inference_sequence();
	let target_derived = recipe::recipe
		.infer()
		.log([recipe::Loss, recipe::Accuracy, recipe::AuPrc]);
	assert_eq!(
		target_derived.log_items(),
		[recipe::Loss, recipe::Accuracy, recipe::AuPrc]
	);
	let error = declaration_error(
		target_derived
			.evaluate()
			.expect_err("target-free inference accepted target-derived metrics"),
	);
	assert_eq!(
		error.kind,
		recipe::DeclarationErrorKind::InvalidInferenceConfiguration,
	);
	for training_only in [recipe::Epoch, recipe::Lr] {
		let (_inference_data, _inference_model) = declare_inference_sequence();
		let error = declaration_error(
			recipe::recipe
				.infer()
				.log(training_only)
				.evaluate()
				.expect_err("inference accepted a training-only metric"),
		);
		assert_eq!(
			error.kind,
			recipe::DeclarationErrorKind::InvalidInferenceConfiguration,
		);
	}
}

#[test]
fn model_bayes_declares_the_requested_ordered_acyclic_graph() {
	recipe::recipe.data("not-opened");
	let model = recipe::recipe
		.model()
		.bayes("housing", ["age", "job"])
		.bayes("loan", ["income", "housing"])
		.bayes("y", ["loan", "housing", "balance"]);

	let declarations = model
		.bayes_dependencies()
		.iter()
		.map(|dependency| {
			(
				dependency.child(),
				dependency
					.parents()
					.iter()
					.map(String::as_str)
					.collect::<Vec<_>>(),
			)
		})
		.collect::<Vec<_>>();
	assert_eq!(
		declarations,
		[
			("housing", vec!["age", "job"]),
			("loan", vec!["income", "housing"]),
			("y", vec!["loan", "housing", "balance"]),
		]
	);
}

#[test]
fn model_bayes_allows_forward_declarations_and_shared_parents() {
	recipe::recipe.data("not-opened");
	let model = recipe::recipe
		.model()
		.bayes("loan", ["income", "housing"])
		.bayes("approval", ["housing", "loan"])
		.bayes("housing", ["employment", "income"]);

	let declarations = model
		.bayes_dependencies()
		.iter()
		.map(|dependency| {
			(
				dependency.child(),
				dependency
					.parents()
					.iter()
					.map(String::as_str)
					.collect::<Vec<_>>(),
			)
		})
		.collect::<Vec<_>>();
	assert_eq!(
		declarations,
		[
			("loan", vec!["income", "housing"]),
			("approval", vec!["housing", "loan"]),
			("housing", vec!["employment", "income"]),
		]
	);
}

#[test]
fn model_bayes_rejects_invalid_names_and_parent_lists() {
	let cases: [(fn(recipe::Model) -> recipe::Model, &str); 4] = [
		(
			|model| model.bayes("", ["age"]),
			"Bayesian child name is empty",
		),
		(
			|model| model.bayes("housing", [""]),
			"Bayesian dependency for \"housing\" contains an empty parent name",
		),
		(
			|model| model.bayes("housing", ["housing"]),
			"Bayesian dependency \"housing\" cannot name itself as a parent",
		),
		(
			|model| model.bayes("housing", ["age", "age"]),
			"Bayesian dependency \"housing\" contains a duplicate parent",
		),
	];
	for (declare_model, detail) in cases {
		let _data = recipe::recipe.data("not-opened");
		let _model = declare_model(recipe::recipe.model());
		let error = declaration_error(
			recipe::recipe
				.infer()
				.evaluate()
				.expect_err("invalid Bayesian declaration was accepted"),
		);
		assert_eq!(error.kind, recipe::DeclarationErrorKind::InvalidBayes);
		assert_eq!(error.detail, detail);
	}
}

#[test]
fn model_bayes_rejects_a_second_declaration_for_the_same_child() {
	recipe::recipe.data("not-opened");
	let model = recipe::recipe
		.model()
		.bayes("housing", ["age"])
		.bayes("housing", ["job"]);

	let error = declaration_error(
		recipe::recipe
			.infer()
			.evaluate()
			.expect_err("duplicate Bayesian child was accepted"),
	);
	assert_eq!(error.kind, recipe::DeclarationErrorKind::InvalidBayes);
	assert_eq!(
		error.detail,
		"Bayesian child \"housing\" is declared more than once"
	);
	assert_eq!(model.bayes_dependencies().len(), 1);
}

#[test]
fn model_bayes_rejects_cycles_completed_through_forward_references() {
	recipe::recipe.data("not-opened");
	let model = recipe::recipe
		.model()
		.bayes("loan", ["housing"])
		.bayes("approval", ["loan"])
		.bayes("housing", ["approval"]);

	let error = declaration_error(
		recipe::recipe
			.infer()
			.evaluate()
			.expect_err("cyclic Bayesian network was accepted"),
	);
	assert_eq!(error.kind, recipe::DeclarationErrorKind::InvalidBayes);
	assert_eq!(error.detail, "Bayesian dependencies contain a cycle");
	assert_eq!(model.bayes_dependencies().len(), 2);
}

#[test]
fn model_load_is_a_recipe_model_chain_method() {
	let _data = recipe::recipe.data("not-opened");
	let model = recipe::recipe.model().load("weights.ogdl");

	assert_eq!(model.weights_source(), Some("weights.ogdl"));

	let _data = recipe::recipe.data("not-opened");
	let _invalid = recipe::recipe.model().load("");
	let error = declaration_error(
		recipe::recipe
			.infer()
			.evaluate()
			.expect_err("empty model checkpoint path was accepted"),
	);
	assert_eq!(error.kind, recipe::DeclarationErrorKind::EmptyValue,);

	let conflicting_declarations: [fn(recipe::Model) -> recipe::Model; 4] = [
		|model| model.layer(1),
		|model| model.bayes("loan", ["income"]),
		|model| model.loss(recipe::mse),
		|model| model.grad(recipe::clip(1.0)),
	];
	for declare_conflict in conflicting_declarations {
		let _data = recipe::recipe.data("not-opened");
		let _model = declare_conflict(recipe::recipe.model().load("weights.ogdl"));
		let error = declaration_error(
			recipe::recipe
				.infer()
				.evaluate()
				.expect_err("checkpoint-backed model accepted another declaration"),
		);
		assert_eq!(error.kind, recipe::DeclarationErrorKind::InvalidLayer);
		assert_eq!(
			error.detail,
			"a checkpoint-backed model cannot also declare layers, Bayesian dependencies, a loss, or gradient policy"
		);
	}
	for declare_conflict in conflicting_declarations {
		let _data = recipe::recipe.data("not-opened");
		let _model = declare_conflict(recipe::recipe.model()).load("weights.ogdl");
		let error = declaration_error(
			recipe::recipe
				.infer()
				.evaluate()
				.expect_err("a declared model accepted a checkpoint source"),
		);
		assert_eq!(error.kind, recipe::DeclarationErrorKind::InvalidLayer);
		assert_eq!(
			error.detail,
			"a checkpoint-backed model cannot also declare layers, Bayesian dependencies, a loss, or gradient policy"
		);
	}

	let _data = recipe::recipe.data("not-opened");
	let _model = recipe::recipe
		.model()
		.load("first.ogdl")
		.load("second.ogdl");
	let error = declaration_error(
		recipe::recipe
			.infer()
			.evaluate()
			.expect_err("a second checkpoint source was accepted"),
	);
	assert_eq!(error.kind, recipe::DeclarationErrorKind::InvalidLayer);
	assert_eq!(
		error.detail,
		"a model accepts exactly one checkpoint source"
	);
}

#[test]
fn inference_requires_a_fresh_data_then_model_sequence() {
	let missing_data = std::thread::spawn(|| {
		declaration_error(
			recipe::recipe
				.infer()
				.evaluate()
				.expect_err("inference without data was accepted"),
		)
	})
	.join()
	.expect("missing-data diagnostic thread");
	assert_eq!(
		missing_data.kind,
		recipe::DeclarationErrorKind::InvalidInferenceConfiguration,
	);
	assert_eq!(
		missing_data.detail,
		"recipe.infer().evaluate() requires a preceding recipe.data(...) declaration"
	);

	let missing_model = std::thread::spawn(|| {
		let _data = recipe::recipe.data("not-opened");
		declaration_error(
			recipe::recipe
				.infer()
				.evaluate()
				.expect_err("inference without a model was accepted"),
		)
	})
	.join()
	.expect("missing-model diagnostic thread");
	assert_eq!(
		missing_model.kind,
		recipe::DeclarationErrorKind::InvalidInferenceConfiguration,
	);
	assert_eq!(
		missing_model.detail,
		"recipe.infer().evaluate() requires a preceding recipe.model() declaration"
	);

	let missing_after_compile_boundary = std::thread::spawn(|| {
		recipe::recipe.data("not-opened");
		recipe::recipe.model().load("model.gguf");
		recipe::recipe
			.infer()
			.evaluate()
			.expect_err("GGUF architecture execution unexpectedly succeeded");
		declaration_error(
			recipe::recipe
				.infer()
				.evaluate()
				.expect_err("a consumed inference sequence was reusable"),
		)
	})
	.join()
	.expect("compile-boundary consumption diagnostic thread");
	assert_eq!(
		missing_after_compile_boundary.detail,
		"recipe.infer().evaluate() requires a preceding recipe.data(...) declaration"
	);

	let missing_after_policy_error = std::thread::spawn(|| {
		recipe::recipe.data("not-opened");
		recipe::recipe.model().layer(1);
		let policy_error = declaration_error(
			recipe::recipe
				.infer()
				.log(recipe::Loss)
				.evaluate()
				.expect_err("target-derived inference logging was accepted"),
		);
		let reuse_error = declaration_error(
			recipe::recipe
				.infer()
				.evaluate()
				.expect_err("a policy-rejected inference sequence was reusable"),
		);
		(policy_error, reuse_error)
	})
	.join()
	.expect("policy-error consumption diagnostic thread");
	assert_eq!(
		missing_after_policy_error.0.kind,
		recipe::DeclarationErrorKind::InvalidInferenceConfiguration,
	);
	assert_eq!(
		missing_after_policy_error.1.detail,
		"recipe.infer().evaluate() requires a preceding recipe.data(...) declaration"
	);

	let missing_after_incomplete_sequence = std::thread::spawn(|| {
		recipe::recipe.data("not-opened");
		declaration_error(
			recipe::recipe
				.infer()
				.evaluate()
				.expect_err("inference without a model was accepted"),
		);
		recipe::recipe.model().layer(1);
		declaration_error(
			recipe::recipe
				.infer()
				.evaluate()
				.expect_err("an incomplete inference sequence retained its data"),
		)
	})
	.join()
	.expect("incomplete-consumption diagnostic thread");
	assert_eq!(
		missing_after_incomplete_sequence.detail,
		"recipe.infer().evaluate() requires a preceding recipe.data(...) declaration"
	);

	let data_mutation_error = std::thread::spawn(|| {
		let data = recipe::recipe.data("first.csv");
		recipe::recipe.model().layer(1);
		data.set("second.csv");
		declaration_error(
			recipe::recipe
				.infer()
				.evaluate()
				.expect_err("data changed after its model without requiring a new model"),
		)
	})
	.join()
	.expect("data-mutation ordering diagnostic thread");
	assert_eq!(
		data_mutation_error.detail,
		"recipe.infer().evaluate() requires a preceding recipe.model() declaration"
	);
}

#[test]
fn model_preserves_each_declared_block_kind_and_width() {
	let model = recipe::recipe
		.model()
		.perc(1)
		.perc(30)
		.layer(30)
		.conv(30, 3)
		.embed(12)
		.vocab(2048)
		.attn(4)
		.rnn(30)
		.gru(30)
		.lstm(30);

	assert_eq!(
		model.layers(),
		[
			recipe::LayerSpec::Perc {
				count: 1,
				operations: vec![],
			},
			recipe::LayerSpec::Perc {
				count: 30,
				operations: vec![],
			},
			recipe::LayerSpec::Dense {
				units: 30,
				operations: vec![],
			},
			recipe::LayerSpec::Convolution {
				filters: 30,
				kernel: 3,
				activation: recipe::Activation::Linear,
			},
			recipe::LayerSpec::Embedding {
				dimensions: 12,
				vocabulary: Some(2048),
			},
			recipe::LayerSpec::Attention { heads: 4 },
			recipe::LayerSpec::Rnn {
				width: 30,
				operations: vec![],
			},
			recipe::LayerSpec::Gru {
				width: 30,
				operations: vec![],
			},
			recipe::LayerSpec::Lstm {
				width: 30,
				operations: vec![],
			},
		]
	);
}

#[test]
fn model_preserves_convolution_pooling_and_boosted_tree_composition() {
	let model = recipe::recipe
		.model()
		.conv(12, 3)
		.relu()
		.pool(2)
		.conv(24, 6)
		.relu()
		.lgbm(4)
		.cbst(5)
		.xgbst(6)
		.forest(40)
		.lgbm(7)
		.forest(50)
		.cbst(8)
		.forest(60)
		.xgbst(9);

	assert_eq!(
		model.layers(),
		[
			recipe::LayerSpec::Convolution {
				filters: 12,
				kernel: 3,
				activation: recipe::Activation::Relu,
			},
			recipe::LayerSpec::Pool {
				size: 2,
				group_to_neuron: None,
			},
			recipe::LayerSpec::Convolution {
				filters: 24,
				kernel: 6,
				activation: recipe::Activation::Relu,
			},
			recipe::LayerSpec::Lgbm { depth: 4 },
			recipe::LayerSpec::Cbst { depth: 5 },
			recipe::LayerSpec::Xgbst { depth: 6 },
			recipe::LayerSpec::Forest {
				trees: 40,
				booster: Some(recipe::ForestBooster::Lgbm { depth: 7 }),
			},
			recipe::LayerSpec::Forest {
				trees: 50,
				booster: Some(recipe::ForestBooster::Cbst { depth: 8 }),
			},
			recipe::LayerSpec::Forest {
				trees: 60,
				booster: Some(recipe::ForestBooster::Xgbst { depth: 9 }),
			},
		]
	);
}

#[test]
fn kmeans_followed_by_an_equal_width_layer_records_the_exact_identity_mapping() {
	recipe::recipe.data("not-opened");
	let model = recipe::recipe.model().kmeans(4).layer(4);

	let connection = match &model.layers()[0] {
		recipe::LayerSpec::KMeans {
			clusters: 4,
			group_to_neuron: Some(connection),
		} => *connection,
		other => panic!("unexpected kmeans declaration: {other:?}"),
	};
	assert_eq!(connection.groups(), recipe::GroupCount::Exact(4));
	assert_eq!(connection.neurons(), 4);
	assert_eq!(
		connection.routing(4),
		Some(recipe::GroupToNeuronRouting::Identity { width: 4 })
	);
}

#[test]
fn knn_declares_one_terminal_all_output_feature_reduction_form() {
	let data = recipe::recipe.data("not-opened");
	let reduction = recipe::recipe
		.model()
		.knn(5)
		.norm(recipe::layer_norm)
		.silu();

	assert_eq!(
		reduction.layers(),
		[recipe::LayerSpec::Knn {
			neighbors: 5,
			operations: vec![],
		}]
	);
	let error = recipe::compile_knn_model(&recipe::recipe.train(), &data, &reduction)
		.expect_err("post-KNN operations were accepted");
	assert!(matches!(
		error,
		recipe::TrainingError::Declaration(error)
			if error.kind == recipe::DeclarationErrorKind::InvalidLayer
	));
	let _data = recipe::recipe.data("not-opened");
	let _model = recipe::recipe.model().knn(0);
	let error = declaration_error(
		recipe::recipe
			.infer()
			.evaluate()
			.expect_err("zero KNN neighborhood was accepted"),
	);
	assert_eq!(error.kind, recipe::DeclarationErrorKind::InvalidLayer);
}

#[test]
fn residual_branch_preserves_order_width_and_projection_rule() {
	recipe::recipe.data("not-opened");
	let model = recipe::recipe
		.model()
		.residual([recipe::layer(64), recipe::relu()])
		.relu()
		.layer(1);

	assert_eq!(
		model.layers(),
		[
			recipe::LayerSpec::Residual {
				branch: vec![
					recipe::ResidualOperation::Layer { width: 64 },
					recipe::ResidualOperation::Activation(recipe::Activation::Relu),
				],
				output_width: 64,
				skip: recipe::ResidualSkip::IdentityOrLinearProjection,
				operations: vec![recipe::LayerOperation::Activation(recipe::Activation::Relu)],
			},
			recipe::LayerSpec::Dense {
				units: 1,
				operations: vec![],
			},
		]
	);
	let preactivated = recipe::recipe.model().residual([
		recipe::relu(),
		recipe::layer(32),
		recipe::relu(),
		recipe::layer(64),
	]);
	assert!(matches!(
		preactivated.layers(),
		[recipe::LayerSpec::Residual {
			branch,
			output_width: 64,
			..
		}] if branch == &[
			recipe::ResidualOperation::Activation(recipe::Activation::Relu),
			recipe::ResidualOperation::Layer { width: 32 },
			recipe::ResidualOperation::Activation(recipe::Activation::Relu),
			recipe::ResidualOperation::Layer { width: 64 },
		]
	));
	assert!(matches!(
		recipe::recipe.model().residual(recipe::layer(64)).layers(),
		[recipe::LayerSpec::Residual {
			output_width: 64,
			..
		}]
	));

	let invalid_declarations: [fn(recipe::Model) -> recipe::Model; 3] = [
		|model| model.residual([] as [recipe::ResidualOperation; 0]),
		|model| model.residual([recipe::layer(0), recipe::relu()]),
		|model| model.residual(recipe::relu()),
	];
	for declare_invalid in invalid_declarations {
		let _data = recipe::recipe.data("not-opened");
		let _model = declare_invalid(recipe::recipe.model());
		let error = declaration_error(
			recipe::recipe
				.infer()
				.evaluate()
				.expect_err("invalid residual branch was accepted"),
		);
		assert_eq!(error.kind, recipe::DeclarationErrorKind::InvalidLayer,);
	}
}

#[test]
fn residual_training_compiles_without_flattening_the_branch() {
	let csv = Fixture::write("csv", b"feature,target\n1,2\n2,3\n3,4\n4,5\n");
	let data = recipe::recipe
		.data(csv.path().to_str().unwrap())
		.target("target")
		.norm(recipe::z_score)
		.split(0.5);
	let model = recipe::recipe
		.model()
		.residual([recipe::relu(), recipe::layer(64), recipe::relu()])
		.norm(recipe::layer_norm)
		.silu()
		.layer(1)
		.loss(recipe::mse);
	let policy = recipe::recipe
		.train()
		.optimizer(recipe::adamw)
		.epochs(1)
		.lr(0.001)
		.cos();

	let compiled = recipe::compile_training(&policy, &data, &model).unwrap();
	assert_eq!(compiled.blocks().len(), 2);
	let recipe_training::DenseBlock::Residual(residual) = &compiled.blocks()[0] else {
		panic!("public residual was flattened into an ordinary dense layer");
	};
	assert_eq!(residual.output_width().unwrap().get(), 64);
	assert_eq!(
		residual.branch(),
		[
			recipe_training::DenseResidualOperation::Operation(recipe_training::DenseOperation::Activation(
				recipe_training::DenseActivation::Relu,
			)),
			recipe_training::DenseResidualOperation::Layer(recipe_training::DenseLayer::with_operations(
				core::num::NonZeroU64::new(64).unwrap(),
				[],
			)),
			recipe_training::DenseResidualOperation::Operation(recipe_training::DenseOperation::Activation(
				recipe_training::DenseActivation::Relu,
			)),
		]
	);
	assert_eq!(
		residual.operations(),
		[
			recipe_training::DenseOperation::Normalization(recipe_training::DenseNormalization::Layer),
			recipe_training::DenseOperation::Activation(recipe_training::DenseActivation::Silu),
		]
	);
	compiled.graph().validate().unwrap();
	compiled.program().validate().unwrap();
}

#[test]
fn pool_training_compiles_as_a_structured_block_with_grouped_dense_routing() {
	let csv = Fixture::write(
		"csv",
		b"a,b,c,d,target\n\
		1,4,3,2,10\n\
		2,5,4,3,11\n\
		3,6,5,4,12\n\
		4,7,6,5,13\n\
		5,8,7,6,14\n\
		6,9,8,7,15\n\
		7,10,9,8,16\n\
		8,11,10,9,17\n",
	);
	let data = recipe::recipe
		.data(csv.path().to_str().unwrap())
		.target("target")
		.norm(recipe::z_score)
		.split(0.75);
	let model = recipe::recipe
		.model()
		.pool(2)
		.layer(4)
		.layer(1)
		.loss(recipe::mse);
	let policy = recipe::recipe
		.train()
		.optimizer(recipe::adamw)
		.epochs(1)
		.lr(0.001)
		.cos();

	let compiled = recipe::compile_training(&policy, &data, &model).unwrap();
	assert_eq!(compiled.blocks().len(), 3);
	let recipe_training::DenseBlock::Pool(pool) = &compiled.blocks()[0] else {
		panic!("public pool was flattened into an ordinary dense layer");
	};
	assert_eq!(pool.size().get(), 2);
	assert_eq!(pool.group_to_neuron().unwrap().get(), 4);
	assert!(matches!(
		&compiled.blocks()[1],
		recipe_training::DenseBlock::Layer(layer) if layer.width().get() == 4
	));

	let checkpoint = recipe_training::CheckpointManifest::from_compiled(&compiled).unwrap();
	assert_eq!(checkpoint.format_version(), 9);
	compiled.graph().validate().unwrap();
	compiled.program().validate().unwrap();
}

#[test]
fn kmeans_training_compiles_as_a_stateful_distance_reduction() {
	let csv = Fixture::write(
		"csv",
		b"a,b,c,d,target\n\
		1,4,3,2,10\n\
		2,5,4,3,11\n\
		3,6,5,4,12\n\
		4,7,6,5,13\n\
		5,8,7,6,14\n\
		6,9,8,7,15\n\
		7,10,9,8,16\n\
		8,11,10,9,17\n",
	);
	let data = recipe::recipe
		.data(csv.path().to_str().unwrap())
		.target("target")
		.norm(recipe::z_score)
		.split(0.75);
	let model = recipe::recipe
		.model()
		.kmeans(3)
		.layer(3)
		.layer(1)
		.loss(recipe::mse);
	let policy = recipe::recipe
		.train()
		.optimizer(recipe::adamw)
		.epochs(2)
		.lr(0.001)
		.cos();

	let compiled = recipe::compile_training(&policy, &data, &model).unwrap();
	assert!(matches!(
		&compiled.blocks()[0],
		recipe_training::DenseBlock::KMeans(kmeans)
			if kmeans.clusters().get() == 3 && kmeans.group_to_neuron().unwrap().get() == 3
	));
	let recipe_training::DenseBlockState::KMeans(state) = &compiled.outputs().blocks[0] else {
		panic!("K-means training did not retain centroid state");
	};
	assert_eq!(state.input_width.get(), 4);
	assert_eq!(state.clusters.get(), 3);
	assert_ne!(state.initial_centroids, state.updated_centroids);
	let checkpoint = recipe_training::CheckpointManifest::from_compiled(&compiled).unwrap();
	assert_eq!(checkpoint.format_version(), 10);
	compiled.graph().validate().unwrap();
	compiled.program().validate().unwrap();
}

#[test]
fn kmeans_training_preserves_divisible_and_fully_connected_dense_routing() {
	let csv = Fixture::write(
		"csv",
		b"a,b,c,d,target\n\
		1,4,3,2,0\n\
		2,5,4,3,1\n\
		3,6,5,4,0\n\
		4,7,6,5,1\n\
		5,8,7,6,0\n\
		6,9,8,7,1\n\
		7,10,9,8,0\n\
		8,11,10,9,1\n",
	);
	let data = recipe::recipe
		.data(csv.path().to_str().unwrap())
		.target("target")
		.norm(recipe::z_score)
		.split(0.75);
	let policy = recipe::recipe
		.train()
		.optimizer(recipe::adamw)
		.epochs(1)
		.lr(0.001);
	let cases = [
		(
			4,
			4,
			recipe_training::DenseGroupToNeuronRouting::Identity {
				width: NonZeroU64::new(4).unwrap(),
			},
		),
		(
			4,
			8,
			recipe_training::DenseGroupToNeuronRouting::Expand {
				groups: NonZeroU64::new(4).unwrap(),
				neurons: NonZeroU64::new(8).unwrap(),
				neurons_per_group: NonZeroU64::new(2).unwrap(),
			},
		),
		(
			8,
			4,
			recipe_training::DenseGroupToNeuronRouting::Contract {
				groups: NonZeroU64::new(8).unwrap(),
				neurons: NonZeroU64::new(4).unwrap(),
				groups_per_neuron: NonZeroU64::new(2).unwrap(),
			},
		),
		(
			5,
			8,
			recipe_training::DenseGroupToNeuronRouting::FullyConnected {
				groups: NonZeroU64::new(5).unwrap(),
				neurons: NonZeroU64::new(8).unwrap(),
			},
		),
	];

	for (clusters, neurons, expected) in cases {
		let model = recipe::recipe
			.model()
			.kmeans(clusters)
			.layer(neurons)
			.layer(1)
			.loss(recipe::bce);
		let compiled = recipe::compile_training(&policy, &data, &model).unwrap();
		let recipe_training::DenseBlock::KMeans(kmeans) = &compiled.blocks()[0] else {
			panic!("first block was not K-means");
		};
		assert_eq!(kmeans.routing(), Some(expected));
		assert_eq!(
			recipe_training::CheckpointManifest::from_compiled(&compiled)
				.unwrap()
				.format_version(),
			10
		);
		compiled.graph().validate().unwrap();
		compiled.program().validate().unwrap();
	}
}

#[test]
fn convolution_training_compiles_public_repeated_channel_topology_without_flattening() {
	let csv = Fixture::write(
		"csv",
		b"a,b,c,d,target\n\
		1,2,3,4,0\n\
		2,3,4,5,1\n\
		3,4,5,6,0\n\
		4,5,6,7,1\n\
		5,6,7,8,1\n\
		6,7,8,9,0\n\
		7,8,9,10,1\n\
		8,9,10,11,0\n",
	);
	let data = recipe::recipe
		.data(csv.path().to_str().unwrap())
		.target("target")
		.norm(recipe::z_score)
		.split(0.75);
	let model = recipe::recipe
		.model()
		.conv(2, 2)
		.relu()
		.conv(3, 2)
		.prelu()
		.pool(2)
		.layer(1)
		.loss(recipe::bce);
	let policy = recipe::recipe
		.train()
		.optimizer(recipe::adamw)
		.epochs(1)
		.lr(0.001)
		.cos();

	let compiled = recipe::compile_training(&policy, &data, &model).unwrap();
	let [
		recipe_training::DenseBlock::Convolution(first),
		recipe_training::DenseBlock::Convolution(second),
		recipe_training::DenseBlock::Pool(pool),
		recipe_training::DenseBlock::Layer(output),
	] = compiled.blocks()
	else {
		panic!("public convolution topology was flattened or reordered");
	};
	assert_eq!((first.kernel().get(), first.filters().get()), (2, 2));
	assert_eq!(
		first.operations(),
		[recipe_training::DenseOperation::Activation(
			recipe_training::DenseActivation::Relu,
		)]
	);
	assert_eq!((second.kernel().get(), second.filters().get()), (2, 3));
	assert_eq!(
		second.operations(),
		[recipe_training::DenseOperation::Activation(
			recipe_training::DenseActivation::PRelu,
		)]
	);
	assert_eq!(pool.size().get(), 2);
	assert_eq!(output.width().get(), 1);
	let checkpoint = recipe_training::CheckpointManifest::from_compiled(&compiled).unwrap();
	assert_eq!(checkpoint.format_version(), 9);
	compiled.graph().validate().unwrap();
	compiled.program().validate().unwrap();
}

#[test]
fn residual_training_routes_binary_and_multiclass_validation() {
	let binary_csv = Fixture::write(
		"csv",
		b"feature,target\n1,0\n2,1\n3,0\n4,1\n5,1\n6,0\n7,0\n8,1\n",
	);
	let binary_data = recipe::recipe
		.data(binary_csv.path().to_str().unwrap())
		.target("target")
		.norm(recipe::z_score)
		.split(0.75);
	let binary_model = recipe::recipe
		.model()
		.residual([recipe::layer(4), recipe::relu()])
		.layer(1)
		.loss(recipe::bce);
	let binary_policy = recipe::recipe
		.train()
		.optimizer(recipe::adamw)
		.epochs(1)
		.lr(0.001)
		.cos()
		.log(recipe::AuPrc);
	let binary = recipe::compile_training(&binary_policy, &binary_data, &binary_model).unwrap();
	assert!(binary.outputs().validation.is_some());
	assert!(binary.outputs().multiclass_validation.is_none());

	let multiclass_csv = Fixture::write(
		"csv",
		b"feature,target\n\
		1,red\n\
		2,blue\n\
		3,green\n\
		4,red\n\
		5,blue\n\
		6,green\n\
		7,red\n\
		8,blue\n\
		9,green\n\
		10,red\n\
		11,blue\n\
		12,green\n",
	);
	let multiclass_data = recipe::recipe
		.data(multiclass_csv.path().to_str().unwrap())
		.target("target")
		.norm(recipe::z_score)
		.split(0.75);
	let multiclass_model = recipe::recipe
		.model()
		.residual([recipe::relu(), recipe::layer(4), recipe::relu()])
		.silu()
		.layer(4)
		.loss(recipe::ce);
	let multiclass_policy = recipe::recipe
		.train()
		.optimizer(recipe::adamw)
		.epochs(1)
		.lr(0.001)
		.cos()
		.log(recipe::Accuracy);
	let multiclass = recipe::compile_training(&multiclass_policy, &multiclass_data, &multiclass_model).unwrap();
	assert!(multiclass.outputs().validation.is_none());
	assert!(multiclass.outputs().multiclass_validation.is_some());
	assert!(matches!(
		multiclass.blocks()[0],
		recipe_training::DenseBlock::Residual(_)
	));
	assert_eq!(multiclass.output_adapter(), None);
}

#[test]
fn grouped_blocks_record_divisible_and_fully_connected_dense_routing() {
	recipe::recipe.data("not-opened");
	let cases = [
		(
			recipe::recipe.model().kmeans(4).layer(8),
			4,
			recipe::GroupCount::Exact(4),
			recipe::GroupToNeuronRouting::Expand {
				groups: 4,
				neurons: 8,
				neurons_per_group: 2,
			},
		),
		(
			recipe::recipe.model().kmeans(8).layer(4),
			8,
			recipe::GroupCount::Exact(8),
			recipe::GroupToNeuronRouting::Contract {
				groups: 8,
				neurons: 4,
				groups_per_neuron: 2,
			},
		),
		(
			recipe::recipe.model().kmeans(5).layer(8),
			5,
			recipe::GroupCount::Exact(5),
			recipe::GroupToNeuronRouting::FullyConnected {
				groups: 5,
				neurons: 8,
			},
		),
		(
			recipe::recipe.model().pool(2).layer(8),
			5,
			recipe::GroupCount::Derived,
			recipe::GroupToNeuronRouting::FullyConnected {
				groups: 5,
				neurons: 8,
			},
		),
	];

	for (model, resolved_groups, declared_groups, expected_routing) in cases {
		let connection = match &model.layers()[0] {
			recipe::LayerSpec::KMeans {
				group_to_neuron: Some(connection),
				..
			}
			| recipe::LayerSpec::Pool {
				group_to_neuron: Some(connection),
				..
			} => *connection,
			other => panic!("unexpected grouped declaration: {other:?}"),
		};
		assert_eq!(connection.groups(), declared_groups);
		assert_eq!(connection.routing(resolved_groups), Some(expected_routing));
	}
}

#[test]
fn model_blocks_keep_post_block_operations_and_reject_zero_widths() {
	recipe::recipe.data("not-opened");
	let operations = vec![
		recipe::LayerOperation::Normalization(recipe::LayerNormalization::LayerNorm),
		recipe::LayerOperation::Activation(recipe::Activation::Silu),
	];
	let model = recipe::recipe
		.model()
		.perc(30)
		.norm(recipe::layer_norm)
		.silu()
		.rnn(30)
		.norm(recipe::layer_norm)
		.silu()
		.gru(30)
		.norm(recipe::layer_norm)
		.silu()
		.lstm(30)
		.norm(recipe::layer_norm)
		.silu();

	assert_eq!(
		model.layers(),
		[
			recipe::LayerSpec::Perc {
				count: 30,
				operations: operations.clone(),
			},
			recipe::LayerSpec::Rnn {
				width: 30,
				operations: operations.clone(),
			},
			recipe::LayerSpec::Gru {
				width: 30,
				operations: operations.clone(),
			},
			recipe::LayerSpec::Lstm {
				width: 30,
				operations,
			},
		]
	);
	let invalid_declarations: [fn(recipe::Model) -> recipe::Model; 17] = [
		|model| model.perc(0),
		|model| model.rnn(0),
		|model| model.gru(0),
		|model| model.lstm(0),
		|model| model.conv(0, 3),
		|model| model.conv(3, 0),
		|model| model.pool(0),
		|model| model.lgbm(0),
		|model| model.cbst(0),
		|model| model.xgbst(0),
		|model| model.forest(0).lgbm(3),
		|model| model.forest(3),
		|model| model.kmeans(0),
		|model| model.embed(0),
		|model| model.embed(8).vocab(0),
		|model| model.vocab(8),
		|model| model.attn(0),
	];
	for declare_invalid in invalid_declarations {
		let _data = recipe::recipe.data("not-opened");
		let _model = declare_invalid(recipe::recipe.model());
		let error = declaration_error(
			recipe::recipe
				.infer()
				.evaluate()
				.expect_err("zero-width block was accepted"),
		);
		assert_eq!(error.kind, recipe::DeclarationErrorKind::InvalidLayer,);
	}
}

#[test]
fn facade_defers_validation_without_hidden_side_effects() {
	let invalid_data = recipe::recipe.data("").split(1.0);
	let invalid_model = recipe::recipe.model().layer(0).relu();
	let error = match recipe::compile_training(
		&recipe::recipe.train().epochs(0),
		&invalid_data,
		&invalid_model,
	) {
		Err(recipe::TrainingError::Declaration(error)) => error,
		_ => panic!("invalid declaration must fail closed"),
	};
	assert_eq!(
		error.kind,
		recipe::DeclarationErrorKind::InvalidTrainingConfiguration
	);
	let data = recipe::recipe.data("never-opened.csv");
	let model = recipe::recipe.model().layer(1).loss(recipe::bce);
	let error = match recipe::compile_training(&recipe::recipe.train(), &data, &model) {
		Err(recipe::TrainingError::Unsupported { detail }) => detail,
		_ => panic!("incomplete training declaration was accepted"),
	};
	assert_eq!(
		error,
		"dense training requires the Recipe-owned AdamW optimizer on the training declaration"
	);
}

#[test]
fn binary_exposes_the_automatic_probe_command() {
	let output = std::process::Command::new(env!("CARGO_BIN_EXE_recipe"))
		.args(["probe", "--help"])
		.output()
		.expect("run Recipe CLI help");
	assert!(output.status.success());
	let stdout = String::from_utf8(output.stdout).expect("help is UTF-8");
	assert!(stdout.contains("recipe probe"));
}

#[test]
fn binary_help_lists_run_probe_and_structural_model_conversion() {
	let output = std::process::Command::new(env!("CARGO_BIN_EXE_recipe"))
		.arg("--help")
		.output()
		.expect("run Recipe CLI help");
	assert!(output.status.success());
	assert_eq!(
		String::from_utf8(output.stdout).expect("help is UTF-8"),
		"Usage:\n\trecipe run FILE.rs\t[ARGS...]\n\trecipe probe\t\t[OPTIONS]\n\trecipe convert INPUT OUTPUT\n",
	);
}

#[test]
fn binary_converts_gguf_to_structural_ogdl_and_back_byte_identically() {
	let root = FixtureDirectory::new();
	let source = root.path().join("gapped.gguf");
	let structural = root.path().join("gapped.ogdl");
	let rebuilt = root.path().join("rebuilt.gguf");
	let mut bytes = b"GGUF".to_vec();
	bytes.extend_from_slice(&3_u32.to_le_bytes());
	bytes.extend_from_slice(&1_u64.to_le_bytes());
	bytes.extend_from_slice(&0_u64.to_le_bytes());
	bytes.extend_from_slice(&1_u64.to_le_bytes());
	bytes.push(b'x');
	bytes.extend_from_slice(&1_u32.to_le_bytes());
	bytes.extend_from_slice(&1_u64.to_le_bytes());
	bytes.extend_from_slice(&24_u32.to_le_bytes());
	bytes.extend_from_slice(&4096_u64.to_le_bytes());
	bytes.resize(64 + 4096, 0);
	bytes.push(0x5a);
	std::fs::write(&source, &bytes).unwrap();

	let encode = std::process::Command::new(env!("CARGO_BIN_EXE_recipe"))
		.args(["convert"])
		.arg(&source)
		.arg(&structural)
		.output()
		.expect("convert GGUF to structural OGDL");
	assert!(
		encode.status.success(),
		"{}",
		String::from_utf8_lossy(&encode.stderr)
	);
	let text = std::fs::read_to_string(&structural).unwrap();
	assert!(text.contains("schema recipe-gguf-structural-v1"));
	assert!(text.contains(&format!("file_bytes {}", bytes.len())));
	assert!(
		text.len() < bytes.len(),
		"large binary alignment gap was not compacted structurally"
	);
	assert!(!text.contains("base64"));
	assert!(!text.contains("raw_bytes"));
	assert_eq!(
		std::fs::metadata(&structural).unwrap().permissions().mode() & 0o777,
		0o600
	);

	let decode = std::process::Command::new(env!("CARGO_BIN_EXE_recipe"))
		.args(["convert"])
		.arg(&structural)
		.arg(&rebuilt)
		.output()
		.expect("convert structural OGDL to GGUF");
	assert!(
		decode.status.success(),
		"{}",
		String::from_utf8_lossy(&decode.stderr)
	);
	assert_eq!(std::fs::read(rebuilt).unwrap(), bytes);
	assert_eq!(
		std::fs::metadata(root.path().join("rebuilt.gguf"))
			.unwrap()
			.permissions()
			.mode() & 0o777,
		0o600
	);
}

#[test]
fn binary_conversion_never_overwrites_and_removes_failed_partial_outputs() {
	let root = FixtureDirectory::new();
	let source = root.path().join("invalid.gguf");
	let existing = root.path().join("existing.ogdl");
	let partial = root.path().join("partial.ogdl");
	std::fs::write(&source, b"GGUF").unwrap();
	std::fs::write(&existing, b"keep me").unwrap();

	let overwrite = std::process::Command::new(env!("CARGO_BIN_EXE_recipe"))
		.args(["convert"])
		.arg(&source)
		.arg(&existing)
		.output()
		.expect("reject conversion overwrite");
	assert!(!overwrite.status.success());
	assert_eq!(std::fs::read(&existing).unwrap(), b"keep me");

	let failed = std::process::Command::new(env!("CARGO_BIN_EXE_recipe"))
		.args(["convert"])
		.arg(&source)
		.arg(&partial)
		.output()
		.expect("reject invalid GGUF conversion");
	assert!(!failed.status.success());
	assert!(
		!partial.exists(),
		"failed conversion retained a partial output"
	);
}

#[test]
fn binary_run_resolves_relative_paths_from_the_source_directory() {
	let root = FixtureDirectory::new();
	let source_directory = root.path().join("source");
	let shell_directory = root.path().join("shell");
	let datasets = source_directory.join("datasets");
	std::fs::create_dir(&source_directory).unwrap();
	std::fs::create_dir(&shell_directory).unwrap();
	std::fs::create_dir(&datasets).unwrap();
	std::fs::write(datasets.join("input.txt"), "source-relative").unwrap();
	let absolute_input = root.path().join("absolute-input.txt");
	std::fs::write(&absolute_input, "absolute").unwrap();
	std::fs::write(
		source_directory.join("train.rs"),
		r#"
fn main() {
	let mut arguments = std::env::args_os().skip(1);
	let absolute_input = arguments.next().expect("absolute input argument");
	let marker = arguments.next().expect("marker argument").into_string().expect("UTF-8 marker");
	assert!(arguments.next().is_none());
	assert_eq!(std::fs::read_to_string("datasets/input.txt").unwrap(), "source-relative");
	assert_eq!(std::fs::read_to_string(absolute_input).unwrap(), "absolute");
	std::fs::write("checkpoint.ogdl", marker).unwrap();
	std::fs::write(
		"current-directory.txt",
		std::env::current_dir().unwrap().display().to_string(),
	)
	.unwrap();
}
"#,
	)
	.unwrap();

	let cache = root.path().join("cache");
	let output = std::process::Command::new(env!("CARGO_BIN_EXE_recipe"))
		.arg("run")
		.arg(Path::new("..").join("source").join("train.rs"))
		.arg(&absolute_input)
		.arg("argument with spaces")
		.current_dir(&shell_directory)
		.env("XDG_CACHE_HOME", &cache)
		.output()
		.expect("run source through Recipe CLI");
	assert!(
		output.status.success(),
		"stdout: {}\nstderr: {}",
		String::from_utf8_lossy(&output.stdout),
		String::from_utf8_lossy(&output.stderr),
	);
	assert_eq!(
		std::fs::read_to_string(source_directory.join("checkpoint.ogdl")).unwrap(),
		"argument with spaces",
	);
	assert_eq!(
		std::fs::read_to_string(source_directory.join("current-directory.txt")).unwrap(),
		std::fs::canonicalize(&source_directory)
			.unwrap()
			.display()
			.to_string(),
	);
	assert!(!shell_directory.join("checkpoint.ogdl").exists());
	assert_eq!(
		std::fs::read_dir(cache.join("recipe-next/run"))
			.unwrap()
			.count(),
		0,
		"compiled source-run binary was not cleaned up",
	);
}

#[test]
fn binary_run_selects_recipe_data_and_knn_in_one_syntactic_lowering() {
	let root = FixtureDirectory::new();
	let source_directory = root.path().join("source");
	std::fs::create_dir(&source_directory).unwrap();
	std::fs::write(
		source_directory.join("sibling.rs"),
		"pub fn marker() -> usize { 17 }\n",
	)
	.unwrap();
	std::fs::write(
		source_directory.join("train.rs"),
		r#"use recipe::*;
mod sibling;

struct Other;

impl Other {
	fn data(&self) -> usize { 11 }
	fn knn(&self, left: usize, right: usize) -> usize { left + right }
}

fn main() {
	assert_eq!(Other.data(), 11);
	assert_eq!(Other.knn(2, 3), 5);
	assert_eq!(sibling::marker(), 17);
	let data = recipe.data().set("first.csv").set("second.zip");
	assert_eq!(data.sources(), ["first.csv", "second.zip"]);
	let model: Model = recipe.model();
	let reduction = model.knn(5);
	assert_eq!(
		reduction.layers(),
		[LayerSpec::Knn {
			neighbors: 5,
			operations: vec![],
		}],
	);
	let preactivated = recipe.model().residual(relu(), layer(32), relu(), layer(64));
	assert!(matches!(
		preactivated.layers(),
		[LayerSpec::Residual { branch, output_width: 64, .. }]
			if branch == &[
				ResidualOperation::Activation(Activation::Relu),
				ResidualOperation::Layer { width: 32 },
				ResidualOperation::Activation(Activation::Relu),
				ResidualOperation::Layer { width: 64 },
			]
	));
	assert!(matches!(
		recipe.model().residual(layer(16)).layers(),
		[LayerSpec::Residual { output_width: 16, .. }]
	));
	let inference_data = recipe.data("inference.csv");
	let inference_model = recipe.model().load("checkpoint.ogdl");
	let inference = recipe.infer().log(Time);
	assert_eq!(inference_data.sources(), ["inference.csv"]);
	assert_eq!(inference_model.weights_source(), Some("checkpoint.ogdl"));
	assert_eq!(inference.log_items(), [Time]);
}
"#,
	)
	.unwrap();

	let cache = root.path().join("cache");
	let source = source_directory.join("train.rs");
	let output = std::process::Command::new(env!("CARGO_BIN_EXE_recipe"))
		.args(["run", source.to_str().unwrap()])
		.env("XDG_CACHE_HOME", &cache)
		.output()
		.expect("run exact Recipe arities through the source frontend");
	assert!(
		output.status.success(),
		"stdout: {}\nstderr: {}",
		String::from_utf8_lossy(&output.stdout),
		String::from_utf8_lossy(&output.stderr),
	);
	assert!(
		std::fs::read_dir(&source_directory)
			.unwrap()
			.all(|entry| !entry
				.unwrap()
				.file_name()
				.to_string_lossy()
				.contains(".recipe-")),
		"transformed Recipe source was not cleaned up",
	);
	assert_eq!(
		std::fs::read_dir(cache.join("recipe-next/run"))
			.unwrap()
			.count(),
		0,
		"compiled source-run binary was not cleaned up",
	);
}

#[test]
fn binary_run_selects_recipe_residual_without_touching_unrelated_receivers() {
	let root = FixtureDirectory::new();
	let source = root.path().join("train.rs");
	std::fs::write(
		&source,
		r#"use recipe::*;

struct Other;

impl Other {
	fn residual(&self, left: usize, right: usize) -> usize { left + right }
}

fn main() {
	assert_eq!(Other.residual(2, 3), 5);
	let model: Model = recipe.model();
	let model = model.residual(layer(64), relu());
	assert_eq!(
		model.layers(),
		[LayerSpec::Residual {
			branch: vec![
				ResidualOperation::Layer { width: 64 },
				ResidualOperation::Activation(Activation::Relu),
			],
			output_width: 64,
			skip: ResidualSkip::IdentityOrLinearProjection,
			operations: vec![],
		}],
	);
}
"#,
	)
	.unwrap();

	let cache = root.path().join("cache");
	let output = std::process::Command::new(env!("CARGO_BIN_EXE_recipe"))
		.args(["run", source.to_str().unwrap()])
		.env("XDG_CACHE_HOME", &cache)
		.output()
		.expect("run exact Recipe residual syntax through the source frontend");
	assert!(
		output.status.success(),
		"stdout: {}\nstderr: {}",
		String::from_utf8_lossy(&output.stdout),
		String::from_utf8_lossy(&output.stderr),
	);
	assert!(
		std::fs::read_dir(root.path()).unwrap().all(|entry| !entry
			.unwrap()
			.file_name()
			.to_string_lossy()
			.contains(".recipe-")),
		"transformed Recipe source was not cleaned up",
	);
	assert_eq!(
		std::fs::read_dir(cache.join("recipe-next/run"))
			.unwrap()
			.count(),
		0,
		"compiled source-run binary was not cleaned up",
	);
}

#[test]
fn binary_run_maps_lowered_diagnostics_back_to_the_original_source() {
	let root = FixtureDirectory::new();
	let source = root.path().join("train.rs");
	std::fs::write(
		&source,
		r#"use recipe::*;
fn main() {
	let _model = recipe.model().knn(3);
	let _residual = recipe.model().residual(layer(64), relu());
	let _broken: u32 = "not a number";
}
"#,
	)
	.unwrap();

	let output = std::process::Command::new(env!("CARGO_BIN_EXE_recipe"))
		.args(["run", source.to_str().unwrap()])
		.env("XDG_CACHE_HOME", root.path().join("cache"))
		.output()
		.expect("run invalid source through the Recipe frontend");
	assert!(!output.status.success());
	let stderr = String::from_utf8(output.stderr).expect("rustc diagnostics are UTF-8");
	assert!(
		stderr.contains(&format!("{}:5:", source.display())),
		"{stderr}"
	);
	assert!(
		stderr.contains("let _broken: u32 = \"not a number\";"),
		"{stderr}"
	);
	assert!(
		!stderr.contains("residual([layer(64), relu()])"),
		"{stderr}"
	);
	assert!(!stderr.contains(".recipe-"), "{stderr}");
	assert!(
		std::fs::read_dir(root.path()).unwrap().all(|entry| !entry
			.unwrap()
			.file_name()
			.to_string_lossy()
			.contains(".recipe-")),
		"failed transformed Recipe source was not cleaned up",
	);
}

#[test]
fn facade_exposes_the_exact_normative_operation_registry() {
	let registry = recipe::operations::registry();
	assert_eq!(registry.surface_len(), 421);
	assert_eq!(registry.owned_len(), 2);
	assert_eq!(
		recipe::operations::all().len(),
		registry.surface_len() + registry.owned_len()
	);
	assert_eq!(
		recipe::operations::resolve("predict").unwrap_err().kind,
		recipe::operations::OperationErrorKind::AmbiguousSymbol
	);

	let descriptor = recipe::operations::resolve_exact("predict", "catboost-rs/src/lib.rs:811").unwrap();
	assert_eq!(descriptor.symbol, "predict");
	assert_eq!(descriptor.source, "catboost-rs/src/lib.rs:811");
	assert!(descriptor.id.is_duplicate_symbol());
}

#[test]
fn facade_lowers_an_owned_scalar_recipe() {
	let descriptor = recipe::operations::resolve("gpu_add_into").unwrap();
	let program = recipe::operations::lower_scalar(descriptor).unwrap();
	program.validate().unwrap();
	assert_eq!(program.inputs.len(), 2);
	assert_eq!(program.outputs.len(), 1);
}

#[test]
fn facade_lowers_a_verified_primitive_recipe() {
	use recipe::engine::core::{AliasPermission, DType, KernelTemplateId, ValueId};
	use recipe::engine::language::{
		AxisSet, PrimitiveAliasRule, PrimitiveKernel, PrimitiveKind, Reduce, ReduceOperator, ReduceResult, Shape,
		Tensor,
	};

	let input_id = ValueId::new(1);
	let output_id = ValueId::new(2);
	let input = Tensor::contiguous(
		input_id,
		DType::F32,
		Shape::new(vec![2, 2]).unwrap(),
		true,
		false,
	)
	.unwrap();
	let output = Tensor::contiguous(
		output_id,
		DType::F32,
		Shape::new(vec![1]).unwrap(),
		false,
		true,
	)
	.unwrap();
	let tensors = BTreeMap::from([(input_id, &input), (output_id, &output)]);
	let kernel = PrimitiveKernel {
		id: KernelTemplateId::new(1),
		inputs: vec![input_id],
		outputs: vec![output_id],
		alias_rules: vec![PrimitiveAliasRule {
			input: 0,
			output: 0,
			permission: AliasPermission::Forbidden,
		}],
		kind: PrimitiveKind::Reduce(Reduce {
			operator: ReduceOperator::Sum,
			axes: AxisSet::new(vec![0, 1]).unwrap(),
			keep_dimensions: false,
			result: ReduceResult::Value,
			tree_lanes: 64,
		}),
	};
	let request = recipe::operations::PrimitiveRequest {
		kernel: &kernel,
		tensors: &tensors,
	};
	let descriptor = recipe::operations::resolve("gpu_sum_all").unwrap();
	let program = recipe::operations::lower_primitive(
		descriptor,
		request,
		recipe::operations::LoweringHardware {
			subgroup_lanes: 32,
			maximum_workgroup_lanes: 256,
			maximum_shared_memory_per_workgroup: recipe::engine::core::ByteCount::new(64 * 1024),
		},
	)
	.unwrap();
	program.validate().unwrap();
}

#[test]
fn facade_exposes_finite_compositions_without_claiming_scalar_lowering() {
	let descriptor = recipe::operations::resolve("gpu_fft_c2c_1d").unwrap();
	assert!(matches!(
		descriptor.lowering,
		recipe::operations::LoweringAvailability::Composition(_)
	));
	recipe::operations::validate_composition(descriptor).unwrap();
	assert_eq!(
		recipe::operations::lower_scalar(descriptor)
			.unwrap_err()
			.kind,
		recipe::operations::OperationErrorKind::WrongLoweringKind
	);
}

#[test]
fn facade_classifies_every_normative_operation() {
	assert!(recipe::operations::all().all(|descriptor| !matches!(
		descriptor.lowering,
		recipe::operations::LoweringAvailability::Unsupported(_)
	)));
}

#[test]
fn facade_reaches_bounded_raw_model_format_framing() {
	use recipe::engine::ingest::{SafeTensorDType, SafeTensorLimits, parse_safetensors};

	let header = r#"{"weight":{"dtype":"F32","shape":[1],"data_offsets":[0,4]}}"#;
	let mut bytes = Vec::new();
	bytes.extend_from_slice(&u64::try_from(header.len()).unwrap().to_le_bytes());
	bytes.extend_from_slice(header.as_bytes());
	bytes.extend_from_slice(&1.0_f32.to_le_bytes());

	let archive = parse_safetensors(
		&bytes,
		SafeTensorLimits::new(4096, 4096, 16, 8, 128).unwrap(),
	)
	.unwrap();
	assert_eq!(
		archive.entry("weight").map(|entry| entry.dtype()),
		Some(SafeTensorDType::F32)
	);
	assert_eq!(
		archive.encoded_tensor("weight"),
		Some(1.0_f32.to_le_bytes().as_slice())
	);
}

fn run_recipe_source_fixture(source_text: &str) -> (FixtureDirectory, PathBuf, std::process::Output) {
	let root = FixtureDirectory::new();
	let source = root.path().join("train.rs");
	std::fs::write(&source, source_text).expect("write Recipe source-runner fixture");
	let output = std::process::Command::new(env!("CARGO_BIN_EXE_recipe"))
		.args(["run", source.to_str().expect("UTF-8 fixture path")])
		.env("XDG_CACHE_HOME", root.path().join("cache"))
		.output()
		.expect("run Recipe source fixture");
	(root, source, output)
}

fn run_counted_recipe_source_fixture(source_text: &str) -> (FixtureDirectory, PathBuf, std::process::Output, usize) {
	let root = FixtureDirectory::new();
	let source = root.path().join("train.rs");
	std::fs::write(&source, source_text).expect("write counted Recipe source-runner fixture");
	let real_rustc = std::env::split_paths(&std::env::var_os("PATH").expect("PATH is set"))
		.map(|directory| directory.join("rustc"))
		.find(|candidate| candidate.is_file())
		.expect("find rustc on PATH before installing wrapper");
	let count = root.path().join("rustc-invocations");
	let wrapper = root.path().join("rustc-wrapper");
	std::fs::write(
		&wrapper,
		"#!/bin/sh\nprintf 'rustc\\n' >> \"$RECIPE_RUSTC_COUNT\"\nexec \"$RECIPE_REAL_RUSTC\" \"$@\"\n",
	)
	.expect("write rustc counting wrapper");
	let mut permissions = std::fs::metadata(&wrapper)
		.expect("inspect rustc wrapper")
		.permissions();
	permissions.set_mode(0o700);
	std::fs::set_permissions(&wrapper, permissions).expect("make rustc wrapper executable");
	let output = std::process::Command::new(env!("CARGO_BIN_EXE_recipe"))
		.args(["run", source.to_str().expect("UTF-8 fixture path")])
		.env("XDG_CACHE_HOME", root.path().join("cache"))
		.env("RUSTC", &wrapper)
		.env("RECIPE_REAL_RUSTC", &real_rustc)
		.env("RECIPE_RUSTC_COUNT", &count)
		.output()
		.expect("run Recipe source fixture with counted rustc");
	let invocations = std::fs::read_to_string(&count)
		.unwrap_or_default()
		.lines()
		.count();
	(root, source, output, invocations)
}

#[test]
fn binary_run_lowers_all_recipe_only_forms_before_one_rustc_invocation() {
	let (_root, _source, output, invocations) = run_counted_recipe_source_fixture(
		r#"fn main() {
	let data = recipe::recipe.data();
	assert!(data.sources().is_empty());
	let model: recipe::Model = recipe::recipe.model();
	let reduced = model.knn(3);
	assert!(matches!(reduced.layers(), [recipe::LayerSpec::Knn { .. }]));
	let train: recipe::Train = recipe::recipe.train();
	let _save = train.save("model.ogdl", "kernel.hsaco");
	let _resume = recipe::recipe.train().resume("model.ogdl", "kernel.hsaco");
}
"#,
	);
	assert!(
		output.status.success(),
		"stdout: {}\nstderr: {}",
		String::from_utf8_lossy(&output.stdout),
		String::from_utf8_lossy(&output.stderr),
	);
	assert_eq!(invocations, 1, "Recipe source compilation retried rustc");
}

#[test]
fn binary_run_reports_invalid_lowered_source_without_a_rustc_retry() {
	let (_root, source, output, invocations) = run_counted_recipe_source_fixture(
		r#"fn main() {
	let _model = recipe::recipe.model().knn(8, 3);
	let _broken: u32 = "not a number";
}
"#,
	);
	assert!(!output.status.success());
	assert_eq!(invocations, 1, "invalid Recipe source retried rustc");
	let stderr = String::from_utf8(output.stderr).expect("rustc diagnostics are UTF-8");
	assert!(
		stderr.contains(&format!("{}:3:", source.display())),
		"{stderr}"
	);
	assert!(
		stderr.contains("let _broken: u32 = \"not a number\";"),
		"{stderr}"
	);
	assert!(!stderr.contains(".recipe-"), "{stderr}");
}

#[test]
fn binary_run_accepts_named_gradient_clip_with_a_nested_expression() {
	let (root, _source, output) = run_recipe_source_fixture(
		r#"fn main() {
	let untouched = ".grad(clip: 99.0)";
	// .grad(clip: 88.0) must remain a comment.
	let data = recipe::recipe.data().set("first.csv");
	assert_eq!(data.sources(), ["first.csv"]);
	let scale: f64 = 0.25;
	let model = recipe::recipe.model().loss(recipe::bce).grad(clip /* field comment */ : {
		let base = 0.5;
		(base + scale).min(1.0)
	});
	assert_eq!(untouched, ".grad(clip: 99.0)");
	assert_eq!(model.gradient_clip_value(), Some(0.75));
	let canonical = recipe::recipe.model().loss(recipe::mse).grad(recipe::clip(0.5));
	assert_eq!(canonical.gradient_clip_value(), Some(0.5));
}
"#,
	);
	assert!(
		output.status.success(),
		"stdout: {}\nstderr: {}",
		String::from_utf8_lossy(&output.stdout),
		String::from_utf8_lossy(&output.stderr),
	);
	assert!(
		std::fs::read_dir(root.path()).unwrap().all(|entry| !entry
			.unwrap()
			.file_name()
			.to_string_lossy()
			.contains(".recipe-")),
		"named-gradient transformed source was not cleaned up",
	);
}

#[test]
fn binary_run_never_rewrites_named_gradient_syntax_on_an_unrelated_receiver() {
	let (_root, source, output) = run_recipe_source_fixture(
		r#"use recipe::clip;

struct Other;

impl Other {
	fn grad(self, _policy: recipe::Grad) -> Self { self }
}

fn main() {
	let _other = Other.grad(clip: 1.0);
}
"#,
	);
	assert!(!output.status.success());
	let stderr = String::from_utf8(output.stderr).expect("rustc diagnostics are UTF-8");
	assert!(
		stderr.contains(&format!("{}:10:", source.display())),
		"{stderr}"
	);
	assert!(stderr.contains("Other.grad(clip: 1.0)"), "{stderr}");
	assert!(!stderr.contains("Other.grad(clip(1.0))"), "{stderr}");
}

#[test]
fn binary_run_reports_duplicate_named_gradient_fields_at_the_original_source() {
	let (_root, source, output) = run_recipe_source_fixture(
		r#"use recipe::*;
fn main() {
	let _model = recipe.model().loss(bce).grad(clip: 1.0, clip: 2.0);
}
"#,
	);
	assert!(!output.status.success());
	let stderr = String::from_utf8(output.stderr).expect("frontend diagnostic is UTF-8");
	assert!(
		stderr.contains("duplicate named field `clip` in Recipe Model `.grad(...)`"),
		"{stderr}",
	);
	assert!(
		stderr.contains(&format!("{}:3:", source.display())),
		"{stderr}"
	);
	assert!(stderr.contains("clip: 1.0, clip: 2.0"), "{stderr}");
	assert!(!stderr.contains(".recipe-"), "{stderr}");
}

#[test]
fn binary_run_rejects_a_malformed_named_gradient_field_cleanly() {
	let (_root, source, output) = run_recipe_source_fixture(
		r#"use recipe::*;
fn main() {
	let _model = recipe.model().loss(bce).grad(clip:);
}
"#,
	);
	assert!(!output.status.success());
	let stderr = String::from_utf8(output.stderr).expect("frontend diagnostic is UTF-8");
	assert!(
		stderr.contains("malformed Recipe Model `.grad(...)` fields"),
		"{stderr}",
	);
	assert!(stderr.contains("expected exactly `clip: EXPR`"), "{stderr}");
	assert!(
		stderr.contains(&format!("{}:3:", source.display())),
		"{stderr}"
	);
	assert!(stderr.contains("grad(clip:)"), "{stderr}");
	assert!(!stderr.contains(".recipe-"), "{stderr}");
}

#[test]
fn binary_run_maps_named_gradient_followup_diagnostics_to_original_columns() {
	let (_root, source, output) = run_recipe_source_fixture(
		r#"use recipe::*;
fn main() {
	let _model = recipe.model().loss(bce).grad(clip: 1.0);
	let _broken: u32 = "not a number";
}
"#,
	);
	assert!(!output.status.success());
	let stderr = String::from_utf8(output.stderr).expect("rustc diagnostics are UTF-8");
	assert!(
		stderr.contains(&format!("{}:4:21", source.display())),
		"{stderr}"
	);
	assert!(
		stderr.contains("let _broken: u32 = \"not a number\";"),
		"{stderr}"
	);
	assert!(!stderr.contains("grad(clip( 1.0))"), "{stderr}");
	assert!(!stderr.contains(".recipe-"), "{stderr}");
}

#[test]
fn binary_run_maps_diagnostics_inside_the_named_gradient_expression() {
	let (_root, source, output) = run_recipe_source_fixture(
		r#"use recipe::*;
fn main() {
    let _model = recipe.model().loss(bce).grad(clip: missing_norm + 1.0);
}
"#,
	);
	assert!(!output.status.success());
	let stderr = String::from_utf8(output.stderr).expect("rustc diagnostics are UTF-8");
	assert!(
		stderr.contains(&format!("{}:3:54", source.display())),
		"{stderr}"
	);
	assert!(
		stderr.contains("grad(clip: missing_norm + 1.0)"),
		"{stderr}"
	);
	assert!(!stderr.contains("recipe::clip"), "{stderr}");
	assert!(!stderr.contains(".recipe-"), "{stderr}");
}
