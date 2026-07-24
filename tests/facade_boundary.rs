use std::collections::BTreeMap;
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

#[test]
fn facade_reaches_every_entry_point_without_external_work() {
	let data = recipe::data("not-opened-during-construction.csv");
	assert_eq!(data.source(), "not-opened-during-construction.csv");
	assert_eq!(
		recipe::recipe.data("also-not-opened").source(),
		"also-not-opened"
	);

	assert_eq!(recipe::model(), recipe::Model::new());
	assert_eq!(recipe::train(), recipe::Train::new());
	assert_eq!(recipe::infer(), recipe::Infer::new());
	assert_eq!(recipe::recipe.model(), recipe::Model::new());
	assert_eq!(recipe::recipe.train(), recipe::Train::new());
	assert_eq!(recipe::recipe.infer(), recipe::Infer::new());
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
	let declaration = recipe::data(csv.path().to_str().unwrap())
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
		&recipe::data(tsv.path().to_str().unwrap())
			.target("label")
			.split(0.5),
	)
	.unwrap();
	assert_eq!(prepared.train().len(), 2);
	assert_eq!(prepared.validation().len(), 2);
}

#[test]
fn data_bridge_rejects_unrepresentable_declarations_before_io() {
	let multiple = recipe::data("never-opened.csv")
		.set("also-never-opened.csv")
		.target("label")
		.split(0.8);
	assert!(matches!(
		recipe::prepare_data(&multiple),
		Err(recipe::DataPreparationError::UnsupportedSourceCount { count: 2 })
	));

	let no_target = recipe::data("never-opened.csv").split(0.8);
	assert!(matches!(
		recipe::prepare_data(&no_target),
		Err(recipe::DataPreparationError::MissingTargets)
	));

	let no_split = recipe::data("never-opened.csv").target("label");
	assert!(matches!(
		recipe::prepare_data(&no_split),
		Err(recipe::DataPreparationError::MissingSplit)
	));

	let test_source = recipe::data("never-opened.csv")
		.target("label")
		.test("test-never-opened.csv")
		.split(0.8);
	assert!(matches!(
		recipe::prepare_data(&test_source),
		Err(recipe::DataPreparationError::UnsupportedTestSource { .. })
	));
}

#[test]
fn data_bridge_propagates_caller_ingest_bounds() {
	use recipe::engine::ingest::{IngestErrorKind, IngestLimits};

	let fixture = Fixture::write("csv", b"x,label\n1,yes\n2,no\n");
	let declaration = recipe::data(fixture.path().to_str().unwrap())
		.target("label")
		.split(0.5);
	let error = recipe::prepare_data_with_limits(&declaration, IngestLimits::new(4, 8, 4, 16).unwrap()).unwrap_err();
	assert!(matches!(
		error,
		recipe::DataPreparationError::Ingest(ref source)
			if source.kind == IngestErrorKind::SourceLimitExceeded
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
	let declaration = recipe::data(path.to_str().unwrap())
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
	let data = recipe::data(path.to_str().unwrap())
		.target("No-show")
		.exclude(["AppointmentID", "PatientId"])
		.exclude(recipe::cond!(Age < 0))
		.split(0.8);
	let model = recipe::model()
		.norm(recipe::z_score)
		.layer(128)
		.silu()
		.layer(128)
		.silu()
		.layer(1)
		.loss(recipe::bce)
		.optimizer(recipe::adamw);
	let policy = recipe::train()
		.batch_size(2048)
		.epochs(100)
		.lr(0.0001)
		.warmup(5)
		.cosine_decay()
		.gradient_clip(1.0)
		.early_stop(recipe::AuPrc, 10)
		.calibrate(recipe::TemperatureScaling)
		.log([
			recipe::Loss,
			recipe::AuRoc,
			recipe::AuPrc,
			recipe::Brier,
			recipe::CalibrationError,
			recipe::RecallAt(0.10),
			recipe::RecallAt(0.20),
			recipe::RecallAt(0.30),
		]);

	let compiled = recipe::compile_training(&policy, &data, &model).unwrap();
	assert_eq!(compiled.bounds().train_rows, 88_420);
	assert_eq!(compiled.bounds().batch_size, 2048);
	assert_eq!(compiled.bounds().batches_per_epoch, 44);
	assert_eq!(compiled.bounds().training_iterations.get(), 4_400);
	assert_eq!(compiled.bounds().calibration_iterations, 64);
	assert_eq!(compiled.bounds().iterations.get(), 4_464);
	assert_eq!(compiled.program().metrics().len(), 9);
	compiled.graph().validate().unwrap();
	compiled.program().validate().unwrap();
	assert!(!compiled.program().to_ogdl().unwrap().is_empty());
}

#[test]
fn facade_builders_are_immutable_validated_declarations() {
	let base_data = recipe::data("not-opened.csv");
	let prepared_data = base_data
		.clone()
		.set("also-not-opened.csv")
		.target(["label", "weight"])
		.test("test-not-opened.csv")
		.exclude("ignored_*")
		.split(0.8);
	assert_eq!(base_data.sources(), ["not-opened.csv"]);
	assert_eq!(
		prepared_data.sources(),
		["not-opened.csv", "also-not-opened.csv"]
	);
	assert_eq!(prepared_data.targets(), ["label", "weight"]);
	assert_eq!(prepared_data.test_source(), Some("test-not-opened.csv"));
	assert_eq!(prepared_data.exclusions(), ["ignored_*"]);
	assert_eq!(prepared_data.split_fraction(), Some(0.8_f32));

	let model = recipe::model()
		.layer(32)
		.relu()
		.layer(recipe::embed(8).vocab(1024))
		.layer(recipe::attn(4))
		.loss(recipe::mse)
		.lr(0.001);
	let train = recipe::train()
		.epochs(3)
		.log_every(1)
		.log([recipe::Loss, recipe::Accuracy, recipe::Device])
		.plot([recipe::Loss, recipe::Accuracy])
		.net(["local"]);
	let declaration = train
		.declare(&prepared_data, &model)
		.expect("valid declaration");
	assert_eq!(declaration.data(), &prepared_data);
	assert_eq!(declaration.model(), &model);
	assert_eq!(declaration.policy().epoch_bound(), Some(3));
	assert_eq!(declaration.policy().nodes(), ["local"]);

	let inference = recipe::infer()
		.log([recipe::Time])
		.evaluate(&prepared_data, &model)
		.expect("valid inference declaration");
	assert_eq!(inference.data(), Some(&prepared_data));
	assert_eq!(inference.model(), &model);
	assert_eq!(inference.policy().log_items(), [recipe::Time]);
}

#[test]
fn facade_defers_validation_without_hidden_side_effects() {
	let invalid_data = recipe::data("").split(1.0);
	let invalid_model = recipe::model().layer(0).relu().lr(f64::NAN);
	let error = recipe::train()
		.epochs(0)
		.declare(&invalid_data, &invalid_model)
		.expect_err("invalid declaration must fail closed");
	assert_eq!(
		error.kind,
		recipe::DeclarationErrorKind::InvalidTrainingConfiguration
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
	assert!(stdout.contains("theoretical seed contract"));
	assert!(stdout.contains("benchmarks every discovered device and link"));
}

#[test]
fn facade_exposes_the_exact_normative_operation_registry() {
	assert_eq!(recipe::operations::all().len(), 421);
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
	let program = recipe::operations::lower_primitive(descriptor, request).unwrap();
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
