use std::collections::BTreeMap;

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
